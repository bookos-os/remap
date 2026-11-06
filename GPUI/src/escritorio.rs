//! Cliente de los remapeos de `teclas.conf`.
//!
//! En la sesión BookOS los aplica el compositor (`org.bookos.Desktop`, bus de
//! sesión). En cualquier otro escritorio, `bookos-teclasd` (`org.bookos.Teclas`,
//! bus de sistema), que habla el mismo subconjunto de métodos pero solo sabe de
//! teclas. Validar y escribir el fichero es cosa de ellos: aquí no se analiza
//! nada, solo se pasa el JSON de un lado a otro.

use std::collections::HashMap;
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::time::Duration;

use futures_util::StreamExt;
use tokio::sync::{Mutex, OnceCell};
use zbus::zvariant::{OwnedValue, Value};

/// Lo que se espera a que se pulse la tecla. Pasado el plazo se desarma la
/// captura: armada y olvidada, el compositor se tragaría la próxima tecla que
/// se pulsara en cualquier ventana.
const ESPERA_CAPTURA: Duration = Duration::from_secs(10);

/// Número de la captura en curso. Una captura cancelada sigue esperando su
/// plazo; al vencer solo desarma si nadie ha empezado otra, o apagaría la nueva.
static CAPTURA: AtomicU64 = AtomicU64::new(0);
static ARMADA: AtomicBool = AtomicBool::new(false);
static CAMBIO_CAPTURA: Mutex<()> = Mutex::const_new(());
pub fn captura_activa() -> bool {
    ARMADA.load(Ordering::SeqCst)
}

static SESION: OnceCell<zbus::Connection> = OnceCell::const_new();
static SISTEMA: OnceCell<zbus::Connection> = OnceCell::const_new();
/// Se decide una vez: la sesión no cambia de escritorio con la app abierta.
static MOTOR: OnceCell<Motor> = OnceCell::const_new();

#[derive(Clone, Copy)]
enum Motor {
    /// El compositor de BookOS.
    Escritorio,
    /// `bookos-teclasd`.
    Sistema,
}

async fn conectar(motor: Motor) -> zbus::Result<zbus::Proxy<'static>> {
    let (conexion, nombre, ruta) = match motor {
        Motor::Escritorio => (
            SESION.get_or_try_init(zbus::Connection::session).await?,
            "org.bookos.Desktop",
            "/org/bookos/Desktop",
        ),
        Motor::Sistema => (
            SISTEMA.get_or_try_init(zbus::Connection::system).await?,
            "org.bookos.Teclas",
            "/org/bookos/Teclas",
        ),
    };
    zbus::Proxy::new(conexion, nombre, ruta, nombre).await
}

/// El compositor manda si está, porque es el único con las funciones de
/// BookOS. Si `bookos-teclasd` también corre, sigue por debajo con lo suyo.
async fn motor() -> Option<Motor> {
    MOTOR
        .get_or_try_init(|| async {
            if escritorio_sabe_remapear().await {
                Ok(Motor::Escritorio)
            } else if servicio_responde().await {
                Ok(Motor::Sistema)
            } else {
                Err(())
            }
        })
        .await
        .ok()
        .copied()
}

async fn escritorio_sabe_remapear() -> bool {
    let Ok(proxy) = conectar(Motor::Escritorio).await else {
        return false;
    };
    let capacidades: zbus::Result<HashMap<String, OwnedValue>> =
        proxy.call("GetCapabilities", &()).await;
    capacidades.is_ok_and(|c| {
        c.get("key_remap")
            .is_some_and(|v| matches!(&**v, Value::Bool(true)))
    })
}

/// Se llama a un método en vez de mirar si el nombre está en el bus: así D-Bus
/// arranca el servicio si está instalado pero parado.
async fn servicio_responde() -> bool {
    let Ok(proxy) = conectar(Motor::Sistema).await else {
        return false;
    };
    proxy
        .call::<_, _, String>("GetKeyRemaps", &())
        .await
        .is_ok()
}

async fn proxy() -> Result<zbus::Proxy<'static>, String> {
    let motor = motor()
        .await
        .ok_or_else(|| "no hay escritorio BookOS ni bookos-teclasd".to_string())?;
    conectar(motor).await.map_err(|e| e.to_string())
}

/// Quién aplica los remapeos: `"escritorio"`, `"sistema"` o nadie. La interfaz
/// lo usa para avisar de que no hay y para no ofrecer lo que `bookos-teclasd`
/// no sabe hacer.
pub async fn motor_remapeo() -> Option<&'static str> {
    motor().await.map(|motor| match motor {
        Motor::Escritorio => "escritorio",
        Motor::Sistema => "sistema",
    })
}

/// La configuración del escritorio (`GetConfig`) tal cual, en JSON. Solo la
/// tiene el compositor de BookOS. La página solo mira `teclado`, para arrancar
/// con la distribución de la sesión.
pub async fn configuracion_escritorio() -> Result<String, String> {
    proxy()
        .await?
        .call("GetConfig", &())
        .await
        .map_err(|e| e.to_string())
}

pub async fn obtener_remapeos() -> Result<String, String> {
    proxy()
        .await?
        .call("GetKeyRemaps", &())
        .await
        .map_err(|e| e.to_string())
}

pub async fn aplicar_remapeos(json: String) -> Result<(), String> {
    let (ok, error): (bool, String) = proxy()
        .await?
        .call("ApplyKeyRemaps", &(json,))
        .await
        .map_err(|e| e.to_string())?;
    if ok { Ok(()) } else { Err(error) }
}

/// Espera la próxima tecla y devuelve su código evdev y si era Copilot.
/// `None` si se agota el plazo.
pub fn capturar_tecla()
-> impl std::future::Future<Output = Result<Option<(u32, bool)>, String>> + Send {
    let numero = CAPTURA.fetch_add(1, Ordering::SeqCst) + 1;
    ARMADA.store(true, Ordering::SeqCst);
    capturar_numero(numero)
}

async fn capturar_numero(numero: u64) -> Result<Option<(u32, bool)>, String> {
    let proxy = proxy().await?;
    // La suscripción precede al armado para no perder la primera pulsación.
    let mut señales = proxy
        .receive_signal("KeyCaptured")
        .await
        .map_err(|e| e.to_string())?;
    {
        let _cambio = CAMBIO_CAPTURA.lock().await;
        if CAPTURA.load(Ordering::SeqCst) != numero {
            return Ok(None);
        }
        ARMADA.store(true, Ordering::SeqCst);
        let armada: bool = proxy
            .call("CaptureKey", &(true,))
            .await
            .map_err(|e| e.to_string())?;
        if !armada {
            ARMADA.store(false, Ordering::SeqCst);
            return Err("el escritorio no pudo armar la captura".into());
        }
    }
    let resultado = match tokio::time::timeout(ESPERA_CAPTURA, señales.next()).await {
        Ok(Some(mensaje)) => mensaje
            .body()
            .deserialize::<(u32, bool)>()
            .map(Some)
            .map_err(|e| e.to_string()),
        Ok(None) => Err("se cerró la conexión con el escritorio".into()),
        Err(_) => Ok(None),
    };
    let _cambio = CAMBIO_CAPTURA.lock().await;
    if CAPTURA.load(Ordering::SeqCst) == numero {
        // También se desarma ante una señal malformada o una desconexión.
        desarmar(&proxy).await?;
        ARMADA.store(false, Ordering::SeqCst);
    }
    resultado
}

pub fn cancelar_captura() -> impl std::future::Future<Output = Result<(), String>> + Send {
    let numero = CAPTURA.fetch_add(1, Ordering::SeqCst) + 1;
    cancelar_numero(numero)
}

async fn cancelar_numero(numero: u64) -> Result<(), String> {
    let _cambio = CAMBIO_CAPTURA.lock().await;
    if CAPTURA.load(Ordering::SeqCst) == numero {
        desarmar(&proxy().await?).await?;
        ARMADA.store(false, Ordering::SeqCst);
    }
    Ok(())
}

async fn desarmar(proxy: &zbus::Proxy<'_>) -> Result<(), String> {
    let _: bool = proxy
        .call("CaptureKey", &(false,))
        .await
        .map_err(|e| e.to_string())?;
    Ok(())
}

#[cfg(test)]
mod pruebas {
    use super::*;

    #[test]
    #[ignore = "requiere el doble D-Bus en un bus privado; ejecutar tests/probar-bus.sh"]
    fn contrato_del_motor_captura_cancelacion_y_plazo() {
        let base = std::path::PathBuf::from(
            std::env::var_os("REMAP_PRUEBA_BASE").expect("directorio del doble de prueba"),
        );
        let rt = tokio::runtime::Builder::new_multi_thread()
            .worker_threads(1)
            .enable_all()
            .build()
            .expect("ejecutor de prueba");
        rt.block_on(async {
            let esperado = if std::env::var("REMAP_MOTOR").as_deref() == Ok("sistema") {
                "sistema"
            } else {
                "escritorio"
            };
            assert_eq!(motor_remapeo().await, Some(esperado));
            let json = r#"[{"origen":"bloqmayus","destino":"ctrl"}]"#;
            aplicar_remapeos(json.to_owned())
                .await
                .expect("el doble acepta el remapeo");
            let leidos: serde_json::Value = serde_json::from_str(
                &obtener_remapeos()
                    .await
                    .expect("el doble devuelve el estado"),
            )
            .expect("JSON del doble");
            assert_eq!(leidos[0]["destino"], "ctrl");
            std::fs::write(base.join("rechazar"), "").expect("archivo de control del doble");
            assert_eq!(
                aplicar_remapeos("[]".into()).await,
                Err("rechazado_por_prueba".into())
            );
            std::fs::remove_file(base.join("rechazar")).expect("retirar el control del doble");
            std::fs::write(
                base.join("capturar.json"),
                r#"{"codigo":193,"copilot":true}"#,
            )
            .expect("señal de prueba");
            assert_eq!(
                capturar_tecla().await.expect("se recibe la señal"),
                Some((193, true))
            );
            assert!(!captura_activa());
            // Cancelar antes de que el ejecutor sondee la captura no debe armarla después.
            let pendiente = capturar_tecla();
            cancelar_captura().await.expect("cancelación aceptada");
            assert_eq!(pendiente.await.expect("captura invalidada"), None);
            assert!(!captura_activa());
            assert_eq!(
                capturar_tecla()
                    .await
                    .expect("se agota el plazo de diez segundos"),
                None
            );
            assert_eq!(
                std::fs::read_to_string(base.join("armada")).expect("estado del doble"),
                "False"
            );
        });
    }
}
