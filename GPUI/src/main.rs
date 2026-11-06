mod escritorio;
mod gesto;
mod idioma;
mod leyendas;
mod marco;
#[cfg(feature = "pruebas-ui")]
mod pruebas_ui;
mod teclado;

use bookos_ui::controles::{self, EstiloBoton};
use bookos_ui::entrada::{self, Confirmado, Entrada};
use bookos_ui::{Recursos, Tema, estado, icono};
use gpui::{
    App, Context, Entity, FocusHandle, Focusable, FontWeight, IntoElement, KeyDownEvent,
    SharedString, Window, div, img, prelude::*, px, relative, size,
};
use idioma::Idioma;
use marco as ventana;
use serde::{Deserialize, Serialize};
use std::{
    collections::{HashMap, HashSet},
    sync::Arc,
};
use teclado::{Remapeo, Tecla};

const APP_ID: &str = "bookos-teclas";

fn boton(
    tm: &Tema,
    id: impl Into<gpui::ElementId>,
    texto: impl Into<SharedString>,
    estilo: EstiloBoton,
) -> gpui::Stateful<gpui::Div> {
    let color = tm.blue;
    let primario = matches!(estilo, EstiloBoton::Primario);
    controles::boton(tm, id, texto, estilo)
        .when(primario, |d| d.text_color(tm.on_accent))
        .tab_index(0)
        .focus_visible(move |s| s.border_2().border_color(color))
}

fn recursos(ruta: &str) -> Option<&'static [u8]> {
    macro_rules! archivos {($($n:literal),*) => {match ruta {$(concat!("iconos/",$n,".svg") => Some(include_bytes!(concat!("../assets/iconos/",$n,".svg")).as_slice()),)* _=>None}};}
    archivos!(
        "arrow-left",
        "arrow-right",
        "arrow-up",
        "arrow-down",
        "trash",
        "tecla",
        "bookos",
        "cmd",
        "nada"
    )
}

#[derive(Serialize, Deserialize)]
#[serde(default)]
struct Preferencias {
    distribucion: String,
    #[serde(rename = "bookos.dynamicPalette", default = "activado")]
    paleta_dinamica: bool,
}
fn activado() -> bool {
    true
}
impl Default for Preferencias {
    fn default() -> Self {
        Self {
            distribucion: String::new(),
            paleta_dinamica: true,
        }
    }
}

#[derive(Clone, Copy, PartialEq)]
enum Motor {
    Escritorio,
    Sistema,
}
enum Servicio {
    Cargando,
    Disponible(Motor),
    Ausente,
    SinLectura,
}
enum FalloAplicacion {
    Rechazo(String),
    Lectura(String),
    NoCoincide(Vec<Remapeo>),
}
enum Dialogo {
    Captura(u64),
    Quitar(String),
}

struct Ayuda {
    tema: Tema,
    lineas: Vec<String>,
}
impl Render for Ayuda {
    fn render(&mut self, _: &mut Window, _: &mut Context<Self>) -> impl IntoElement {
        div()
            .px(px(14.))
            .py(px(10.))
            .rounded(px(14.))
            .bg(self.tema.card)
            .border_1()
            .border_color(self.tema.brd)
            .shadow(bookos_ui::tema::sombra_menu())
            .text_color(self.tema.tx)
            .text_size(px(12.))
            .max_w(px(300.))
            .children(self.lineas.iter().enumerate().map(|(i, s)| {
                div()
                    .when(i > 0, |d| {
                        d.text_color(self.tema.tx2).text_size(px(11.)).mt(px(4.))
                    })
                    .child(s.clone())
            }))
    }
}

struct Vista {
    tema: Tema,
    tema_del_escritorio: Option<bool>,
    efectos_reducidos: bool,
    idioma: Idioma,
    preferencias: Preferencias,
    distribucion: String,
    leyendas: HashMap<u32, [String; 3]>,
    servicio: Servicio,
    remapeos: Vec<Remapeo>,
    pendientes: Option<Vec<Remapeo>>,
    guardando: bool,
    seleccion: Option<String>,
    desplegado: bool,
    comando: Option<Entity<Entrada>>,
    dialogo: Option<Dialogo>,
    captura: u64,
    pulsadas: HashSet<u32>,
    mayusculas: Option<bool>,
    error: Option<String>,
    runtime: Arc<tokio::runtime::Runtime>,
    foco: FocusHandle,
    foco_cancelar: FocusHandle,
    foco_confirmar: FocusHandle,
    gesto: gesto::Gesto,
    recorrido: f32,
    gesto_generacion: u64,
    ultimo_gesto: Option<std::time::Instant>,
    ignorar_gesto: Option<std::time::Instant>,
    _subs: Vec<gpui::Subscription>,
}

impl Vista {
    fn new(
        window: &mut Window,
        cx: &mut Context<Self>,
        runtime: Arc<tokio::runtime::Runtime>,
    ) -> Self {
        let idioma = Idioma::detectar();
        let preferencias: Preferencias = estado::cargar(APP_ID);
        let distribucion = teclado::distribucion_xkb(&preferencias.distribucion)
            .unwrap_or(if idioma == Idioma::Ingles { "us" } else { "es" })
            .to_owned();
        let foco = cx.focus_handle();
        window.focus(&foco, cx);
        let subs = vec![
            cx.observe_window_appearance(window, |v, w, cx| v.recargar_tema(w, cx)),
            cx.observe_window_activation(window, |v, w, cx| {
                if w.is_window_active() {
                    v.recargar_tema(w, cx);
                    v.sincronizar_apariencia(cx);
                } else {
                    v.pulsadas.clear();
                    cx.notify();
                }
            }),
        ];
        let mut v = Self {
            tema: Tema::para_ventana("auto", window),
            tema_del_escritorio: None,
            efectos_reducidos: false,
            idioma,
            preferencias,
            distribucion,
            leyendas: HashMap::new(),
            servicio: Servicio::Cargando,
            remapeos: Vec::new(),
            pendientes: None,
            guardando: false,
            seleccion: None,
            desplegado: false,
            comando: None,
            dialogo: None,
            captura: 0,
            pulsadas: HashSet::new(),
            mayusculas: None,
            error: None,
            runtime,
            foco,
            foco_cancelar: cx.focus_handle(),
            foco_confirmar: cx.focus_handle(),
            gesto: gesto::Gesto::default(),
            recorrido: 0.,
            gesto_generacion: 0,
            ultimo_gesto: None,
            ignorar_gesto: None,
            _subs: subs,
        };
        v.recargar_tema(window, cx);
        v.cargar_leyendas(cx);
        v.cargar(cx);
        v
    }
    fn error_humano(&self, e: &str) -> String {
        match e {
            "confirmacion_remapeos" => self.t("error.confirmation"),
            "el escritorio no pudo armar la captura" => self.t("error.capture"),
            "se cerró la conexión con el escritorio" => self.t("error.connection"),
            "no hay escritorio BookOS ni bookos-teclasd" => self.t("unavailable.d"),
            otro => otro.to_owned(),
        }
    }
    fn t(&self, k: &str) -> String {
        self.idioma.t(k)
    }
    fn recargar_tema(&mut self, w: &Window, cx: &mut Context<Self>) {
        self.tema = if let Some(oscuro) = self.tema_del_escritorio {
            if self.preferencias.paleta_dinamica {
                Tema::cargar(oscuro)
            } else if oscuro {
                Tema::oscuro()
            } else {
                Tema::claro()
            }
        } else if self.preferencias.paleta_dinamica {
            Tema::para_ventana("auto", w)
        } else {
            match w.appearance() {
                gpui::WindowAppearance::Dark | gpui::WindowAppearance::VibrantDark => {
                    Tema::oscuro()
                }
                _ => Tema::claro(),
            }
        };
        if let Some(e) = &self.comando {
            e.update(cx, |e, _| {
                e.color_pista = self.tema.tx2;
                e.color_cursor = self.tema.blue;
            });
        }
        cx.notify();
    }
    fn sincronizar_apariencia(&mut self, cx: &mut Context<Self>) {
        if !matches!(self.servicio, Servicio::Disponible(Motor::Escritorio)) {
            return;
        }
        let rt = self.runtime.clone();
        cx.spawn(async move |v, cx| {
            if let Ok(Ok(json)) = rt.spawn(escritorio::configuracion_escritorio()).await
                && let Ok(config) = serde_json::from_str::<serde_json::Value>(&json)
            {
                let _ = v.update(cx, |v, cx| v.aplicar_apariencia(&config, cx));
            }
        })
        .detach();
    }
    fn aplicar_apariencia(&mut self, config: &serde_json::Value, cx: &mut Context<Self>) {
        if let Some(oscuro) = match config["tema_efectivo"].as_str() {
            Some("oscuro") => Some(true),
            Some("claro") => Some(false),
            _ => None,
        } {
            self.tema_del_escritorio = Some(oscuro);
            self.tema = if self.preferencias.paleta_dinamica {
                Tema::cargar(oscuro)
            } else if oscuro {
                Tema::oscuro()
            } else {
                Tema::claro()
            };
            if let Some(e) = &self.comando {
                e.update(cx, |e, _| {
                    e.color_cursor = self.tema.blue;
                    e.color_pista = self.tema.tx2;
                });
            }
        }
        self.efectos_reducidos = config["efectos"].as_str() == Some("reducidos");
        cx.notify();
    }
    fn alternar_paleta(&mut self, w: &Window, cx: &mut Context<Self>) {
        self.preferencias.paleta_dinamica = !self.preferencias.paleta_dinamica;
        self.recargar_tema(w, cx);
        if let Err(e) = estado::guardar(APP_ID, &self.preferencias) {
            self.error = Some(self.idioma.con("toast.error", "e", &e.to_string()));
        }
    }
    fn cargar_leyendas(&mut self, cx: &mut Context<Self>) {
        match leyendas::leyendas(&self.distribucion) {
            Ok(l) => self.leyendas = l,
            Err(e) => self.error = Some(self.idioma.con("toast.layout", "e", &e)),
        }
        cx.notify();
    }
    fn cargar(&mut self, cx: &mut Context<Self>) {
        self.servicio = Servicio::Cargando;
        self.error = None;
        let rt = self.runtime.clone();
        cx.spawn(async move |v, cx| {
            let resultado = rt
                .spawn(async {
                    let motor = escritorio::motor_remapeo().await;
                    let datos = if motor.is_some() {
                        escritorio::obtener_remapeos().await.and_then(|s| {
                            serde_json::from_str::<Vec<Remapeo>>(&s).map_err(|e| e.to_string())
                        })
                    } else {
                        Ok(Vec::new())
                    };
                    let configuracion = if motor == Some("escritorio") {
                        escritorio::configuracion_escritorio()
                            .await
                            .ok()
                            .and_then(|s| serde_json::from_str::<serde_json::Value>(&s).ok())
                    } else {
                        None
                    };
                    (motor, datos, configuracion)
                })
                .await;
            let _ = v.update(cx, |v, cx| {
                match resultado {
                    Ok((motor, datos, configuracion)) => {
                        v.servicio = match motor {
                            Some("escritorio") => Servicio::Disponible(Motor::Escritorio),
                            Some(_) => Servicio::Disponible(Motor::Sistema),
                            None => Servicio::Ausente,
                        };
                        match datos {
                            Ok(r) => v.remapeos = r,
                            Err(e) => {
                                v.servicio = Servicio::SinLectura;
                                v.error =
                                    Some(v.idioma.con("toast.error", "e", &v.error_humano(&e)));
                            }
                        };
                        if let Some(config) = configuracion {
                            if v.preferencias.distribucion.is_empty()
                                && let Some(d) = config["teclado"]
                                    .as_str()
                                    .and_then(teclado::distribucion_xkb)
                            {
                                v.distribucion = d.into();
                                v.cargar_leyendas(cx);
                            }
                            v.aplicar_apariencia(&config, cx);
                        }
                    }
                    Err(e) => {
                        v.servicio = Servicio::Ausente;
                        v.error = Some(v.idioma.con("toast.error", "e", &e.to_string()));
                    }
                }
                cx.notify();
            });
        })
        .detach();
    }
    fn destino(&self, origen: &str) -> Option<&str> {
        self.remapeos
            .iter()
            .find(|r| r.origen == origen)
            .map(|r| r.destino.as_str())
    }
    fn nombre(&self, origen: &str) -> String {
        if origen == "copilot" {
            return self.t("key.copilot");
        }
        if let Some(c) = origen.strip_prefix("evdev:") {
            let etiqueta = c
                .parse()
                .ok()
                .and_then(|c| self.leyendas.get(&c))
                .map(|l| if es_letra(l) { &l[1] } else { &l[0] });
            return match etiqueta.filter(|s| !s.is_empty()) {
                Some(s) => self.idioma.con("key.named", "k", s),
                None => {
                    let dibujada = c.parse::<u32>().ok().filter(|c| *c != 193).and_then(|c| {
                        teclado::construir(self.distribucion != "us")
                            .into_iter()
                            .find(|t| t.codigo == Some(c))
                    });
                    match dibujada.filter(|t| !t.etiqueta.is_empty()) {
                        Some(t) => self.idioma.con(
                            "key.named",
                            "k",
                            &if t.etiqueta.starts_with("lbl.") {
                                self.t(&t.etiqueta)
                            } else {
                                t.etiqueta
                            },
                        ),
                        None => self.idioma.con("key.evdev", "n", c),
                    }
                }
            };
        }
        self.t(&format!("dest.{origen}"))
    }
    fn nombre_destino(&self, d: &str) -> String {
        if let Some(cmd) = d.strip_prefix("cmd:") {
            self.idioma.con("dest.cmd.named", "cmd", cmd)
        } else if let Some(c) = d.strip_prefix("evdev:") {
            self.idioma.con("key.evdev", "n", c)
        } else {
            self.t(&format!("dest.{d}"))
        }
    }
    fn abrir(&mut self, origen: Option<String>, cx: &mut Context<Self>) {
        self.seleccion = origen;
        self.recorrido = 0.;
        self.gesto = gesto::Gesto::default();
        self.gesto_generacion += 1;
        self.comando = None;
        self.desplegado = false;
        if let Some(cmd) = self
            .seleccion
            .as_deref()
            .and_then(|o| self.destino(o))
            .and_then(|d| d.strip_prefix("cmd:"))
            .map(str::to_owned)
        {
            self.editar_comando(&cmd, cx);
        }
        cx.notify();
    }
    fn editar_comando(&mut self, valor: &str, cx: &mut Context<Self>) {
        let pista = self.t("detail.cmd.ph");
        let tm = self.tema;
        let entrada = cx.new(|cx| {
            let mut e = Entrada::new(valor.to_owned(), pista, cx);
            e.color_cursor = tm.blue;
            e.color_pista = tm.tx2;
            e
        });
        cx.subscribe(&entrada, |v, _, _: &Confirmado, cx| v.guardar_comando(cx))
            .detach();
        self.comando = Some(entrada);
    }
    fn guardar_comando(&mut self, cx: &mut Context<Self>) {
        if let Some(e) = &self.comando {
            let cmd = e.read(cx).texto().trim();
            if !cmd.is_empty() {
                let destino = format!("cmd:{cmd}");
                self.elegir(&destino, cx);
            }
        }
    }
    fn elegir(&mut self, destino: &str, cx: &mut Context<Self>) {
        if !matches!(self.servicio, Servicio::Disponible(_)) {
            return;
        }
        let Some(origen) = self.seleccion.as_ref() else {
            return;
        };
        // Conserva la última lista solicitada: los clics rápidos no pierden cambios de otras teclas.
        let mut nuevos = self
            .pendientes
            .take()
            .unwrap_or_else(|| self.remapeos.clone());
        match nuevos.iter_mut().find(|r| &r.origen == origen) {
            Some(r) => r.destino = destino.to_owned(),
            None => nuevos.push(Remapeo {
                origen: origen.clone(),
                destino: destino.to_owned(),
            }),
        };
        self.pendientes = Some(nuevos);
        self.enviar(cx);
    }
    fn enviar(&mut self, cx: &mut Context<Self>) {
        if self.guardando {
            return;
        }
        let Some(nuevos) = self.pendientes.take() else {
            return;
        };
        self.guardando = true;
        self.error = None;
        // La copia es la base de la siguiente edición mientras la primera está en vuelo.
        self.pendientes = Some(nuevos.clone());
        let rt = self.runtime.clone();
        cx.spawn(async move |v, cx| {
            let resultado = rt
                .spawn(async move {
                    for intento in 0..3 {
                        let json = serde_json::to_string(&nuevos)
                            .expect("una lista de pares de cadenas se serializa a JSON");
                        escritorio::aplicar_remapeos(json)
                            .await
                            .map_err(FalloAplicacion::Rechazo)?;
                        let json = escritorio::obtener_remapeos()
                            .await
                            .map_err(FalloAplicacion::Lectura)?;
                        let leidos: Vec<Remapeo> = serde_json::from_str(&json)
                            .map_err(|e| FalloAplicacion::Lectura(e.to_string()))?;
                        if equivalentes(&leidos, &nuevos) {
                            return Ok(leidos);
                        }
                        if intento == 2 {
                            return Err(FalloAplicacion::NoCoincide(leidos));
                        }
                    }
                    unreachable!("el tercer intento devuelve el estado leído")
                })
                .await;
            let _ = v.update(cx, |v, cx| {
                v.guardando = false;
                match resultado {
                    Ok(Ok(leidos)) => {
                        let mas = v
                            .pendientes
                            .as_ref()
                            .is_some_and(|p| !equivalentes(p, &leidos));
                        v.remapeos = leidos;
                        if mas {
                            v.enviar(cx);
                        } else {
                            v.pendientes = None;
                        }
                    }
                    otro => {
                        let error = match otro {
                            Ok(Err(FalloAplicacion::Rechazo(e))) => e,
                            Ok(Err(FalloAplicacion::Lectura(e))) => {
                                // Sin lectura no hay una base fiable para enviar otra lista completa.
                                v.servicio = Servicio::SinLectura;
                                e
                            }
                            Ok(Err(FalloAplicacion::NoCoincide(leidos))) => {
                                v.remapeos = leidos;
                                "confirmacion_remapeos".into()
                            }
                            Err(e) => {
                                v.servicio = Servicio::SinLectura;
                                e.to_string()
                            }
                            Ok(Ok(_)) => {
                                unreachable!("el estado confirmado se maneja en la rama anterior")
                            }
                        };
                        v.pendientes = None;
                        v.error = Some(v.idioma.con("toast.error", "e", &v.error_humano(&error)));
                    }
                }
                cx.notify();
            });
        })
        .detach();
        cx.notify();
    }
    fn capturar(&mut self, cx: &mut Context<Self>) {
        if !matches!(self.servicio, Servicio::Disponible(_)) {
            return;
        }
        self.captura += 1;
        let numero = self.captura;
        self.dialogo = Some(Dialogo::Captura(numero));
        let rt = self.runtime.clone();
        let captura = escritorio::capturar_tecla();
        cx.spawn(async move |v, cx| {
            let resultado = rt.spawn(captura).await;
            let _ = v.update(cx, |v, cx| {
                if !matches!(v.dialogo,Some(Dialogo::Captura(n)) if n==numero) {
                    return;
                }
                v.dialogo = None;
                match resultado {
                    Ok(Ok(Some((c, copilot)))) => v.abrir(
                        Some(if copilot {
                            "copilot".into()
                        } else {
                            teclado::origen(c)
                        }),
                        cx,
                    ),
                    Ok(Ok(None)) => v.error = Some(v.t("toast.timeout")),
                    Ok(Err(e)) => {
                        v.error = Some(v.idioma.con("toast.error", "e", &v.error_humano(&e)))
                    }
                    Err(e) => v.error = Some(v.idioma.con("toast.error", "e", &e.to_string())),
                }
                cx.notify();
            });
        })
        .detach();
        cx.notify();
    }
    fn cancelar(&mut self, cx: &mut Context<Self>) {
        if matches!(self.dialogo, Some(Dialogo::Captura(_))) {
            let rt = self.runtime.clone();
            let cancelacion = escritorio::cancelar_captura();
            cx.spawn(async move |v, cx| {
                let r = rt.spawn(cancelacion).await;
                let error = match r {
                    Ok(Ok(())) => None,
                    Ok(Err(e)) => Some(e),
                    Err(e) => Some(e.to_string()),
                };
                if let Some(e) = error {
                    let _ = v.update(cx, |v, cx| {
                        v.error = Some(v.idioma.con("toast.error", "e", &v.error_humano(&e)));
                        cx.notify();
                    });
                }
            })
            .detach();
        }
        self.dialogo = None;
        cx.notify();
    }
    fn fila(
        &self,
        id: impl Into<gpui::ElementId>,
        titulo: String,
        sub: Option<String>,
    ) -> gpui::Stateful<gpui::Div> {
        div()
            .id(id)
            .relative()
            .min_h(px(48.))
            .px(px(20.))
            .py(px(14.))
            .flex()
            .items_center()
            .gap(px(12.))
            .child(
                div()
                    .flex_1()
                    .min_w_0()
                    .child(
                        div()
                            .text_size(px(14.))
                            .font_weight(FontWeight::MEDIUM)
                            .child(titulo),
                    )
                    .children(sub.map(|s| {
                        div()
                            .mt(px(2.))
                            .text_size(px(11.))
                            .line_height(relative(1.4))
                            .text_color(self.tema.tx2)
                            .child(s)
                    })),
            )
    }
    fn tarjeta(&self) -> gpui::Div {
        div()
            .w_full()
            .mb(px(18.))
            .rounded(px(22.))
            .bg(self.tema.card)
            .shadow(self.tema.sombra_reposo())
            .overflow_hidden()
    }
    fn seccion(&self, clave: &str) -> gpui::Div {
        div()
            .mx(px(20.))
            .mb(px(6.))
            .text_size(px(12.))
            .text_color(self.tema.tx2)
            .font_weight(FontWeight::MEDIUM)
            .child(self.t(clave))
    }
    fn opcion(
        &self,
        id: impl Into<gpui::ElementId>,
        texto: String,
        activa: bool,
    ) -> gpui::Stateful<gpui::Div> {
        let tm = self.tema;
        div()
            .id(id)
            .w_full()
            .px(px(12.))
            .py(px(10.))
            .rounded(px(13.))
            .flex()
            .items_center()
            .gap(px(12.))
            .cursor_pointer()
            .tab_index(0)
            .focus_visible(move |s| s.bg(tm.act))
            .hover(move |s| s.bg(tm.hov))
            .active(move |s| s.bg(tm.act))
            .child(
                div()
                    .size(px(18.))
                    .flex_none()
                    .rounded_full()
                    .border_2()
                    .border_color(if activa { tm.blue } else { tm.toff })
                    .bg(if activa {
                        tm.blue
                    } else {
                        gpui::transparent_black()
                    })
                    .flex()
                    .items_center()
                    .justify_center()
                    .when(activa, |d| {
                        d.child(div().size(px(8.)).rounded_full().bg(tm.on_accent))
                    }),
            )
            .child(
                div()
                    .text_size(px(14.5))
                    .font_weight(if activa {
                        FontWeight::SEMIBOLD
                    } else {
                        FontWeight::MEDIUM
                    })
                    .text_color(if activa { tm.blue } else { tm.tx })
                    .child(texto),
            )
    }
    fn inicio(&self, cx: &mut Context<Self>) -> gpui::Div {
        let tm = self.tema;
        let lista = if self.remapeos.is_empty() {
            self.tarjeta().child(
                div()
                    .px(px(20.))
                    .py(px(18.))
                    .text_size(px(13.))
                    .line_height(relative(1.4))
                    .text_color(tm.tx2)
                    .child(self.t("nav.empty")),
            )
        } else {
            self.tarjeta()
                .children(self.remapeos.iter().enumerate().map(|(i, r)| {
                    let origen = r.origen.clone();
                    div()
                        .id(("remapeo", i))
                        .relative()
                        .tab_index(0)
                        .focus_visible(move |s| s.bg(tm.act))
                        .h(px(58.))
                        .px(px(18.))
                        .flex()
                        .items_center()
                        .gap(px(14.))
                        .cursor_pointer()
                        .hover(move |s| s.bg(tm.hov))
                        .active(move |s| s.bg(tm.act))
                        .when(i > 0, |d| {
                            d.child(
                                div()
                                    .absolute()
                                    .top_0()
                                    .left(px(62.))
                                    .right_0()
                                    .h(px(0.5))
                                    .bg(tm.div),
                            )
                        })
                        .child(
                            img(format!("iconos/{}.svg", teclado::categoria(&r.destino)))
                                .size(px(30.))
                                .flex_none(),
                        )
                        .child(
                            div()
                                .flex_1()
                                .min_w_0()
                                .child(
                                    div()
                                        .truncate()
                                        .text_size(px(14.))
                                        .font_weight(FontWeight::MEDIUM)
                                        .child(self.nombre(&r.origen)),
                                )
                                .child(
                                    div()
                                        .truncate()
                                        .text_size(px(11.))
                                        .text_color(tm.tx2)
                                        .child(self.nombre_destino(&r.destino)),
                                ),
                        )
                        .child(icono("chevron-derecha", 14., tm.tx2))
                        .on_click(cx.listener(move |v, _, _, cx| v.abrir(Some(origen.clone()), cx)))
                }))
        };
        let valor = teclado::DISTRIBUCIONES
            .iter()
            .find(|(id, _)| *id == self.distribucion)
            .map_or("", |(_, n)| *n);
        let preferencias = self
            .tarjeta()
            .child(
                self.fila("idioma", self.t("prefs.lang"), None).child(
                    div()
                        .text_size(px(14.))
                        .text_color(tm.tx2)
                        .child(self.t("prefs.lang.auto")),
                ),
            )
            .child(
                self.fila(
                    "distribucion",
                    self.t("prefs.layout"),
                    Some(self.t("prefs.layout.ds")),
                )
                .border_t_1()
                .border_color(tm.div)
                .child(div().text_size(px(14.)).text_color(tm.tx2).child(valor))
                .child(icono(
                    if self.desplegado {
                        "chevron"
                    } else {
                        "chevron-derecha"
                    },
                    14.,
                    tm.tx2,
                ))
                .tab_index(0)
                .focus_visible(move |s| s.bg(tm.act))
                .cursor_pointer()
                .hover(move |s| s.bg(tm.hov))
                .on_click(cx.listener(|v, _, _, cx| {
                    v.desplegado = !v.desplegado;
                    cx.notify();
                })),
            )
            .when(self.desplegado, |d| {
                d.child(
                    div().px(px(8.)).pb(px(8.)).children(
                        teclado::DISTRIBUCIONES
                            .iter()
                            .enumerate()
                            .map(|(i, &(id, nombre))| {
                                self.opcion(
                                    ("distribucion-opcion", i),
                                    nombre.into(),
                                    id == self.distribucion,
                                )
                                .on_click(cx.listener(
                                    move |v, _, _, cx| {
                                        v.distribucion = id.into();
                                        v.preferencias.distribucion = id.into();
                                        v.desplegado = false;
                                        v.cargar_leyendas(cx);
                                        if let Err(e) = estado::guardar(APP_ID, &v.preferencias) {
                                            v.error = Some(v.idioma.con(
                                                "toast.error",
                                                "e",
                                                &e.to_string(),
                                            ));
                                        }
                                        cx.notify();
                                    },
                                ))
                            }),
                    ),
                )
            })
            .child(
                self.fila("paleta", self.t("prefs.palette"), None)
                    .tab_index(0)
                    .focus_visible(move |s| s.bg(tm.act))
                    .on_key_down(cx.listener(|v, e: &KeyDownEvent, w, cx| {
                        if matches!(e.keystroke.key.as_str(), "enter" | "space") {
                            v.alternar_paleta(w, cx);
                            cx.stop_propagation();
                        }
                    }))
                    .border_t_1()
                    .border_color(tm.div)
                    .child(controles::interruptor(
                        &tm,
                        "paleta-interruptor",
                        self.preferencias.paleta_dinamica,
                        cx.listener(|v, _, w, cx| v.alternar_paleta(w, cx)),
                    )),
            );
        div()
            .child(
                div()
                    .mt(px(2.))
                    .mx(px(4.))
                    .mb(px(18.))
                    .text_size(px(28.))
                    .font_weight(FontWeight::BOLD)
                    .child(self.t("app.title")),
            )
            .child(self.seccion("nav.remaps"))
            .child(lista)
            .child(self.seccion("prefs.title"))
            .child(preferencias)
            .child(self.seccion("about.title"))
            .child(
                self.tarjeta()
                    .child(self.fila(
                        "copilot-info",
                        self.t("about.copilot.t"),
                        Some(self.t("about.copilot.d")),
                    ))
                    .child(
                        self.fila("fn-info", self.t("about.fn.t"), Some(self.t("about.fn.d")))
                            .border_t_1()
                            .border_color(tm.div),
                    )
                    .child(
                        self.fila(
                            "sesion-info",
                            self.t("about.session.t"),
                            Some(self.t(
                                if matches!(self.servicio, Servicio::Disponible(Motor::Sistema)) {
                                    "about.session.d.sistema"
                                } else {
                                    "about.session.d"
                                },
                            )),
                        )
                        .border_t_1()
                        .border_color(tm.div),
                    ),
            )
    }
    fn detalle_tecla(&self, origen: &str, cx: &mut Context<Self>) -> gpui::Div {
        let tm = self.tema;
        let actual = self.destino(origen);
        let en_comando = self.comando.is_some();
        let mut selector = div()
            .w_full()
            .mb(px(18.))
            .p(px(6.))
            .rounded(px(18.))
            .bg(tm.card)
            .border_1()
            .border_color(tm.brd);
        for (g, opciones) in teclado::GRUPOS {
            let opciones: Vec<_> = opciones
                .iter()
                .filter(|o| {
                    !matches!(self.servicio, Servicio::Disponible(Motor::Sistema))
                        || matches!(teclado::categoria(o), "tecla" | "nada")
                })
                .collect();
            if opciones.is_empty() {
                continue;
            }
            selector = selector.child(
                div()
                    .px(px(12.))
                    .pt(px(12.))
                    .pb(px(4.))
                    .text_size(px(12.))
                    .text_color(tm.tx2)
                    .child(self.t(g)),
            );
            for &opcion in opciones {
                let activa = if opcion == "cmd:" {
                    en_comando
                } else {
                    !en_comando && actual == Some(opcion)
                };
                selector = selector.child(
                    self.opcion(
                        SharedString::from(format!("destino-{opcion}")),
                        self.t(&format!("dest.{opcion}")),
                        activa,
                    )
                    .when(!matches!(self.servicio, Servicio::Disponible(_)), |d| {
                        d.opacity(0.4)
                    })
                    .on_click(cx.listener(move |v, evento, w, cx| {
                        if !matches!(v.servicio, Servicio::Disponible(_)) {
                            return;
                        }
                        if opcion == "cmd:" {
                            v.editar_comando("", cx);
                            if let Some(e) = &v.comando {
                                w.focus(&e.focus_handle(cx), cx);
                            }
                        } else {
                            v.comando = None;
                            v.elegir(opcion, cx);
                            if matches!(evento, gpui::ClickEvent::Mouse(_)) {
                                w.focus(&v.foco, cx);
                            }
                        }
                        cx.notify();
                    })),
                );
            }
        }
        let codigo = teclado::codigo(origen).map_or_else(|| "—".into(), |c| c.to_string());
        div()
            .child(
                div()
                    .min_h(px(44.))
                    .mb(px(16.))
                    .flex()
                    .items_center()
                    .gap(px(8.))
                    .child(
                        div()
                            .id("atras")
                            .tab_index(0)
                            .focus_visible(move |s| s.bg(tm.act))
                            .size(px(32.))
                            .rounded(px(8.))
                            .flex()
                            .items_center()
                            .justify_center()
                            .cursor_pointer()
                            .hover(move |s| s.bg(tm.hov))
                            .child(icono("arrow-left", 18., tm.blue))
                            .on_click(cx.listener(|v, _, w, cx| {
                                v.abrir(None, cx);
                                w.focus(&v.foco, cx);
                            })),
                    )
                    .child(
                        div()
                            .text_size(px(22.))
                            .font_weight(FontWeight::BOLD)
                            .child(self.nombre(origen)),
                    ),
            )
            .child(self.tarjeta().child(self.fila(
                "tecla-actual",
                if matches!(self.servicio, Servicio::SinLectura) {
                    "—".into()
                } else {
                    actual.map_or_else(|| self.t("detail.default"), |d| self.nombre_destino(d))
                },
                Some(if origen == "copilot" {
                    self.t("detail.copilot")
                } else {
                    self.idioma.con("detail.code", "c", &codigo)
                }),
            )))
            .child(self.seccion("detail.does"))
            .when(matches!(self.servicio, Servicio::Disponible(_)), |d| {
                d.child(selector)
            })
            .when_some(
                self.comando
                    .as_ref()
                    .filter(|_| matches!(self.servicio, Servicio::Disponible(_))),
                |d, e| {
                    d.child(
                        self.tarjeta()
                            .p(px(12.))
                            .flex()
                            .items_center()
                            .gap(px(10.))
                            .child(
                                div()
                                    .flex_1()
                                    .min_w_0()
                                    .h(px(40.))
                                    .px(px(14.))
                                    .flex()
                                    .items_center()
                                    .rounded(px(14.))
                                    .border_1()
                                    .border_color(tm.brd)
                                    .bg(tm.sbg)
                                    .text_size(px(13.))
                                    .child(e.clone()),
                            )
                            .child(
                                boton(
                                    &tm,
                                    "guardar-comando",
                                    self.t("detail.cmd.save"),
                                    EstiloBoton::Primario,
                                )
                                .on_click(cx.listener(|v, _, _, cx| v.guardar_comando(cx))),
                            ),
                    )
                },
            )
            .when(
                actual.is_some() && matches!(self.servicio, Servicio::Disponible(_)),
                |d| {
                    let origen = origen.to_owned();
                    d.child(
                        self.tarjeta().child(
                            self.fila(
                                "quitar",
                                self.t("detail.remove"),
                                Some(self.t("detail.remove.ds")),
                            )
                            .text_color(tm.red)
                            .tab_index(0)
                            .focus_visible(move |s| s.bg(tm.act))
                            .cursor_pointer()
                            .hover(move |s| s.bg(tm.hov))
                            .child(icono("trash", 16., tm.red))
                            .on_click(cx.listener(
                                move |v, _, w, cx| {
                                    w.focus(&v.foco, cx);
                                    v.dialogo = Some(Dialogo::Quitar(origen.clone()));
                                    cx.notify();
                                },
                            )),
                        ),
                    )
                },
            )
    }
    fn ayuda(&self, t: &Tecla) -> Vec<String> {
        let Some(c) = t.codigo else {
            return vec![self.t("lbl.fn"), self.t("about.fn.d")];
        };
        let o = if c == 193 {
            "copilot".into()
        } else {
            teclado::origen(c)
        };
        let mut lineas = vec![
            self.nombre(&o),
            self.destino(&o).map_or_else(
                || self.t("detail.default"),
                |d| self.idioma.con("kbd.does", "d", &self.nombre_destino(d)),
            ),
            if c == 193 {
                self.t("detail.copilot")
            } else {
                self.idioma.con("detail.code", "c", &c.to_string())
            },
        ];
        if let Some(niveles) = self.leyendas.get(&c) {
            lineas.push(
                ["tip.normal", "tip.shift", "tip.altgr"]
                    .iter()
                    .zip(niveles)
                    .filter(|(_, n)| !n.is_empty())
                    .map(|(k, n)| format!("{} {n}", self.t(k)))
                    .collect::<Vec<_>>()
                    .join(" · "),
            );
        }
        if c == 58
            && let Some(m) = self.mayusculas
        {
            lineas.push(self.t(if m { "caps.on" } else { "caps.off" }));
        }
        lineas
    }
    fn tecla(&self, t: &Tecla, cx: &mut Context<Self>) -> gpui::AnyElement {
        let tm = self.tema;
        let origen = t.codigo.map(|c| {
            if c == 193 {
                "copilot".into()
            } else {
                teclado::origen(c)
            }
        });
        let seleccionada = origen
            .as_ref()
            .is_some_and(|o| self.seleccion.as_ref() == Some(o));
        let cambiada = origen.as_ref().is_some_and(|o| self.destino(o).is_some());
        let pulsada = t.codigo.is_some_and(|c| self.pulsadas.contains(&c));
        let activa = seleccionada || pulsada;
        let bg = if activa {
            tm.blue
        } else if cambiada {
            bookos_ui::tema::mezclar(tm.blue, tm.card, 0.1)
        } else {
            tm.bg
        };
        let fg = if activa {
            tm.on_accent
        } else if cambiada {
            tm.blue
        } else {
            tm.tx
        };
        let mut tecla = div()
            .id(SharedString::from(format!("tecla-{:?}", t.codigo)))
            .relative()
            .size_full()
            .rounded(px(10.))
            .border_1()
            .border_color(if activa {
                tm.blue
            } else if cambiada {
                tm.blue.opacity(0.3)
            } else {
                tm.brd
            })
            .bg(bg)
            .text_color(fg)
            .text_size(px(12.))
            .font_weight(FontWeight::MEDIUM)
            .flex()
            .items_center()
            .justify_center()
            .when(t.codigo.is_none(), |d| d.opacity(0.4));
        if t.etiqueta.is_empty() {
            if let Some(l) = t.codigo.and_then(|c| self.leyendas.get(&c)) {
                if es_letra(l) {
                    tecla = tecla.child(div().child(l[1].clone()));
                } else {
                    tecla = tecla
                        .child(
                            div()
                                .absolute()
                                .left(px(7.))
                                .top(px(6.))
                                .text_size(px(11.))
                                .child(if l[1] != l[0] {
                                    l[1].clone()
                                } else {
                                    String::new()
                                }),
                        )
                        .child(
                            div()
                                .absolute()
                                .left(px(7.))
                                .bottom(px(6.))
                                .text_size(px(11.))
                                .child(l[0].clone()),
                        );
                }
                if !l[2].is_empty() && l[2] != l[0] && l[2] != l[1] && (!es_letra(l) || l[2] == "€")
                {
                    tecla = tecla.child(
                        div()
                            .absolute()
                            .right(px(7.))
                            .bottom(px(6.))
                            .text_size(px(11.))
                            .text_color(if activa { fg } else { tm.tx2 })
                            .child(l[2].clone()),
                    );
                }
            }
        } else if t.etiqueta.starts_with("arrow-") {
            let nombre = match t.etiqueta.as_str() {
                "arrow-left" => "arrow-left",
                "arrow-right" => "arrow-right",
                "arrow-up" => "arrow-up",
                _ => "arrow-down",
            };
            tecla = tecla.child(icono(nombre, 12., fg));
        } else {
            tecla = tecla.child(div().px(px(3.)).truncate().child(
                if t.etiqueta.starts_with("lbl.") {
                    self.t(&t.etiqueta)
                } else {
                    t.etiqueta.clone()
                },
            ));
        }
        if cambiada {
            tecla = tecla.child(
                div()
                    .absolute()
                    .top(px(5.))
                    .right(px(5.))
                    .size(px(6.))
                    .rounded_full()
                    .bg(if activa { fg } else { tm.blue }),
            );
        }
        if t.codigo == Some(58) {
            tecla = tecla.child(
                div()
                    .absolute()
                    .top(px(5.))
                    .right(px(if cambiada { 15. } else { 5. }))
                    .size(px(6.))
                    .rounded_full()
                    .bg(if self.mayusculas == Some(true) {
                        tm.green
                    } else {
                        tm.toff
                    }),
            );
        }
        if t.intro_iso {
            tecla = tecla.items_start().pt(px(14.)).rounded_tl(px(0.)).child(
                div()
                    .absolute()
                    .top(px(-1.))
                    .left(relative(-0.2))
                    .w(relative(0.2))
                    .h(px(48.))
                    .bg(bg)
                    .border_1()
                    .border_r_0()
                    .border_color(if activa { tm.blue } else { tm.brd })
                    .rounded_l(px(10.)),
            );
        }
        let lineas = self.ayuda(t);
        tecla = tecla.tooltip(move |_, cx| {
            cx.new(|_| Ayuda {
                tema: tm,
                lineas: lineas.clone(),
            })
            .into()
        });
        if let Some(o) = origen {
            tecla = tecla
                .tab_index(0)
                .focus_visible(move |s| s.border_2().border_color(tm.blue))
                .cursor_pointer()
                .hover(move |s| {
                    s.bg(if activa {
                        bg
                    } else {
                        bookos_ui::tema::mezclar(tm.tx, bg, 0.06)
                    })
                })
                .on_click(cx.listener(move |v, _, w, cx| {
                    let destino = if v.seleccion.as_ref() == Some(&o) {
                        None
                    } else {
                        Some(o.clone())
                    };
                    v.abrir(destino, cx);
                    w.focus(&v.foco, cx);
                }));
        }
        div()
            .absolute()
            .left(relative(t.columna / 60.))
            .top(px(t.y))
            .w(relative(t.ancho / 60.))
            .h(px(t.alto))
            .pl(px(if t.columna == 0. { 0. } else { 3. }))
            .pr(px(if t.columna + t.ancho == 60. { 0. } else { 3. }))
            .child(tecla)
            .into_any_element()
    }
    fn teclado(&self, cx: &mut Context<Self>) -> gpui::Stateful<gpui::Div> {
        let tm = self.tema;
        div()
            .id("detalle")
            .flex_1()
            .min_w_0()
            .h_full()
            .overflow_y_scroll()
            .px(px(44.))
            .pb(px(32.))
            .child(
                div()
                    .max_w(px(1000.))
                    .child(
                        div()
                            .min_h(px(44.))
                            .mb(px(16.))
                            .flex()
                            .items_center()
                            .justify_between()
                            .gap(px(8.))
                            .child(
                                div()
                                    .text_size(px(22.))
                                    .font_weight(FontWeight::BOLD)
                                    .child(self.t("kbd.title")),
                            )
                            .when(matches!(self.servicio, Servicio::Disponible(_)), |d| {
                                d.child(
                                    boton(
                                        &tm,
                                        "detectar",
                                        self.t("kbd.detect"),
                                        EstiloBoton::Secundario,
                                    )
                                    .bg(tm.card)
                                    .shadow(tm.sombra_reposo())
                                    .on_click(cx.listener(
                                        |v, _, w, cx| {
                                            w.focus(&v.foco, cx);
                                            v.capturar(cx);
                                        },
                                    )),
                                )
                            }),
                    )
                    .child(
                        div()
                            .mb(px(16.))
                            .p(px(20.))
                            .rounded(px(25.))
                            .bg(tm.card)
                            .shadow(tm.sombra_reposo())
                            .child(
                                div().relative().w_full().h(px(304.)).children(
                                    teclado::construir(self.distribucion != "us")
                                        .iter()
                                        .map(|t| self.tecla(t, cx)),
                                ),
                            ),
                    )
                    .child(
                        div()
                            .mx(px(20.))
                            .text_size(px(12.))
                            .line_height(relative(1.5))
                            .text_color(tm.tx2)
                            .child(self.t("kbd.hint")),
                    )
                    .when(matches!(self.servicio, Servicio::Cargando), |d| {
                        d.child(self.aviso(self.t("status.loading"), None, cx))
                    })
                    .when(matches!(self.servicio, Servicio::Ausente), |d| {
                        d.child(self.aviso(
                            self.t("unavailable.t"),
                            Some(self.t("unavailable.d")),
                            cx,
                        ))
                    })
                    .when_some(self.error.as_ref(), |d, e| {
                        d.child(self.aviso(self.t("status.error"), Some(e.clone()), cx))
                    })
                    .when(self.guardando, |d| {
                        d.child(
                            div()
                                .mt(px(16.))
                                .text_size(px(12.))
                                .text_color(tm.tx2)
                                .child(self.t("status.saving")),
                        )
                    }),
            )
    }
    fn aviso(&self, titulo: String, texto: Option<String>, cx: &mut Context<Self>) -> gpui::Div {
        self.tarjeta()
            .mt(px(18.))
            .p(px(20.))
            .child(
                div()
                    .text_size(px(14.))
                    .font_weight(FontWeight::MEDIUM)
                    .child(titulo),
            )
            .children(texto.map(|s| {
                div()
                    .mt(px(6.))
                    .text_size(px(12.))
                    .text_color(self.tema.tx2)
                    .child(s)
            }))
            .when(!matches!(self.servicio, Servicio::Cargando), |d| {
                d.child(
                    boton(
                        &self.tema,
                        "reintentar",
                        self.t("status.retry"),
                        EstiloBoton::Secundario,
                    )
                    .mt(px(12.))
                    .on_click(cx.listener(|v, _, _, cx| v.cargar(cx))),
                )
            })
    }
    fn modal(&self, cx: &mut Context<Self>) -> Option<gpui::AnyElement> {
        let d = self.dialogo.as_ref()?;
        let tm = self.tema;
        let (titulo, mensaje) = match d {
            Dialogo::Captura(_) => (self.t("cap.title"), self.t("cap.msg")),
            Dialogo::Quitar(o) => (
                self.t("remove.title"),
                self.idioma.con("remove.msg", "k", &self.nombre(o)),
            ),
        };
        let mut botones = div().flex().gap(px(10.)).mt(px(20.)).child(
            boton(&tm, "cancelar", self.t("cancel"), EstiloBoton::Secundario)
                .track_focus(&self.foco_cancelar)
                .flex_1()
                .h(px(48.))
                .flex()
                .items_center()
                .justify_center()
                .text_size(px(15.))
                .rounded(px(14.))
                .on_click(cx.listener(|v, _, w, cx| {
                    v.cancelar(cx);
                    w.focus(&v.foco, cx);
                })),
        );
        if let Dialogo::Quitar(o) = d {
            let o = o.clone();
            botones = botones.child(
                boton(
                    &tm,
                    "confirmar-quitar",
                    self.t("remove.ok"),
                    EstiloBoton::Peligro,
                )
                .track_focus(&self.foco_confirmar)
                .flex_1()
                .h(px(48.))
                .flex()
                .items_center()
                .justify_center()
                .text_size(px(15.))
                .rounded(px(14.))
                .on_click(cx.listener(move |v, _, w, cx| {
                    let mut nuevos = v.pendientes.take().unwrap_or_else(|| v.remapeos.clone());
                    nuevos.retain(|r| r.origen != o);
                    v.pendientes = Some(nuevos);
                    v.dialogo = None;
                    v.abrir(None, cx);
                    v.enviar(cx);
                    w.focus(&v.foco, cx);
                })),
            );
        }
        Some(
            div()
                .id("capa-modal")
                .absolute()
                .inset_0()
                .bg(bookos_ui::tema::rgba(0, 0, 0, 0.45))
                .flex()
                .items_center()
                .justify_center()
                .on_mouse_down(gpui::MouseButton::Left, |_, _, cx| cx.stop_propagation())
                .child(
                    div()
                        .w(px(310.))
                        .rounded(px(26.))
                        .px(px(24.))
                        .pt(px(28.))
                        .pb(px(20.))
                        .bg(tm.card)
                        .shadow(bookos_ui::tema::sombra_modal())
                        .when(matches!(d, Dialogo::Captura(_)), |d| {
                            d.child(
                                div()
                                    .flex()
                                    .justify_center()
                                    .mb(px(16.))
                                    .child(img("iconos/tecla.svg").size(px(44.))),
                            )
                        })
                        .child(
                            div()
                                .text_size(px(17.))
                                .font_weight(FontWeight::BOLD)
                                .text_center()
                                .child(titulo),
                        )
                        .child(
                            div()
                                .mt(px(10.))
                                .text_size(px(13.))
                                .line_height(relative(1.5))
                                .text_color(tm.tx2)
                                .text_center()
                                .child(mensaje),
                        )
                        .child(botones),
                )
                .into_any_element(),
        )
    }
    fn desplazar(&mut self, e: &gpui::ScrollWheelEvent, cx: &mut Context<Self>) {
        if self.seleccion.is_none() || self.dialogo.is_some() {
            return;
        }
        let ahora = std::time::Instant::now();
        if self.ignorar_gesto.is_some_and(|t| ahora < t) {
            return;
        }
        if self
            .ultimo_gesto
            .is_none_or(|t| ahora.duration_since(t).as_millis() > 200)
        {
            self.gesto = gesto::Gesto::default();
        }
        self.ultimo_gesto = Some(ahora);
        let delta = e.delta.pixel_delta(px(16.));
        let finalizado = matches!(e.touch_phase, gpui::TouchPhase::Ended)
            || (delta.x == px(0.) && delta.y == px(0.));
        if let Some(r) = self.gesto.mover(f32::from(delta.x), f32::from(delta.y)) {
            self.recorrido = r;
            self.gesto_generacion += 1;
            let generacion = self.gesto_generacion;
            cx.stop_propagation();
            cx.notify();
            cx.spawn(async move |v, cx| {
                cx.background_executor()
                    .timer(std::time::Duration::from_millis(if finalizado {
                        0
                    } else {
                        140
                    }))
                    .await;
                let completar = match v.update(cx, |v, _| {
                    if v.gesto_generacion == generacion {
                        Some(v.recorrido > 0.5)
                    } else {
                        None
                    }
                }) {
                    Ok(Some(c)) => c,
                    _ => return,
                };
                let inicio = std::time::Instant::now();
                let desde = match v.update(cx, |v, _| v.recorrido) {
                    Ok(r) => r,
                    Err(_) => return,
                };
                loop {
                    let t = inicio.elapsed().as_secs_f32();
                    let destino = if completar { 1. } else { 0. };
                    let w = std::f32::consts::TAU / 0.315;
                    let r = destino + (desde - destino) * (1. + w * t) * (-w * t).exp();
                    let seguir = v
                        .update(cx, |v, cx| {
                            if v.gesto_generacion != generacion {
                                return false;
                            }
                            v.recorrido = r;
                            if v.efectos_reducidos || (r - destino).abs() < 0.001 {
                                if completar {
                                    v.abrir(None, cx);
                                } else {
                                    v.recorrido = 0.;
                                }
                                v.gesto = gesto::Gesto::default();
                                v.ignorar_gesto = Some(
                                    std::time::Instant::now()
                                        + std::time::Duration::from_millis(if completar {
                                            800
                                        } else {
                                            450
                                        }),
                                );
                                cx.notify();
                                return false;
                            }
                            cx.notify();
                            true
                        })
                        .unwrap_or(false);
                    if !seguir {
                        break;
                    }
                    cx.background_executor()
                        .timer(std::time::Duration::from_millis(16))
                        .await;
                }
            })
            .detach();
        }
    }
    fn pulsar(&mut self, e: &KeyDownEvent, w: &mut Window, cx: &mut Context<Self>) {
        if let Some(c) = e.keystroke.native_key_code {
            self.pulsadas.insert(u32::from(c));
        }
        if e.keystroke.key == "tab" {
            if self.dialogo.is_some() {
                let confirmar = matches!(self.dialogo, Some(Dialogo::Quitar(_)))
                    && self.foco_cancelar.is_focused(w);
                w.focus(
                    if confirmar {
                        &self.foco_confirmar
                    } else {
                        &self.foco_cancelar
                    },
                    cx,
                );
            } else if e.keystroke.modifiers.shift {
                w.focus_prev(cx);
            } else {
                w.focus_next(cx);
            }
            cx.stop_propagation();
        }
        if e.keystroke.key == "escape" && self.dialogo.is_none() {
            if self.desplegado {
                self.desplegado = false;
            } else {
                self.abrir(None, cx);
            }
            w.focus(&self.foco, cx);
        }
        cx.notify();
    }
}
fn es_letra(l: &[String; 3]) -> bool {
    !l[0].is_empty() && l[0] != l[1] && l[0].to_uppercase() == l[1]
}
fn equivalentes(a: &[Remapeo], b: &[Remapeo]) -> bool {
    let normalizar = |r: &Remapeo| {
        (
            teclado::codigo(&r.origen).map_or_else(|| r.origen.clone(), teclado::origen),
            teclado::codigo(&r.destino).map_or_else(|| r.destino.clone(), teclado::origen),
        )
    };
    let mut a: Vec<_> = a.iter().map(normalizar).collect();
    let mut b: Vec<_> = b.iter().map(normalizar).collect();
    a.sort();
    b.sort();
    a == b
}
impl Render for Vista {
    fn render(&mut self, w: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let tm = self.tema;
        let entidad = cx.entity().downgrade();
        let lateral = div()
            .id("lateral")
            .w(px(384.))
            .flex_none()
            .h_full()
            .px(px(24.))
            .pb(px(32.))
            .overflow_y_scroll()
            .child(gesto::desplazar(
                match self.seleccion.as_deref() {
                    Some(o) => self.detalle_tecla(o, cx),
                    None => self.inicio(cx),
                },
                28. * self.recorrido,
            ));
        let lateral = div()
            .relative()
            .w(px(384.))
            .h_full()
            .flex_none()
            .overflow_hidden()
            .child(lateral)
            .when(self.recorrido > 0., |d| {
                d.child(
                    div().absolute().inset_0().child(gesto::desplazar(
                        div()
                            .w(px(384.))
                            .h_full()
                            .bg(tm.bg)
                            .px(px(24.))
                            .pb(px(32.))
                            .child(self.inicio(cx)),
                        -384. * (1. - self.recorrido),
                    )),
                )
            });
        let contenido = div()
            .size_full()
            .flex()
            .child(lateral)
            .child(self.teclado(cx));
        let modal = self.modal(cx);
        ventana::marco(&tm, w, None, None, contenido, modal)
            .id("raiz")
            .tab_group()
            .track_focus(&self.foco)
            .child(
                gpui::canvas(
                    |_, _, _| (),
                    move |_, _, w, _| {
                        w.on_mouse_event(move |e: &gpui::ScrollWheelEvent, fase, _, cx| {
                            if fase.capture() {
                                let _ = entidad.update(cx, |v, cx| v.desplazar(e, cx));
                            }
                        });
                    },
                )
                .absolute()
                .inset_0(),
            )
            .capture_key_down(cx.listener(Self::pulsar))
            .capture_key_up(cx.listener(|v, e: &gpui::KeyUpEvent, _, cx| {
                if let Some(c) = e.keystroke.native_key_code {
                    v.pulsadas.remove(&u32::from(c));
                }
                cx.notify();
            }))
            .on_modifiers_changed(cx.listener(|v, e: &gpui::ModifiersChangedEvent, _, cx| {
                v.mayusculas = Some(e.capslock.on);
                cx.notify();
            }))
    }
}
fn main() {
    let runtime = Arc::new(
        tokio::runtime::Builder::new_multi_thread()
            .worker_threads(1)
            .enable_all()
            .build()
            .expect("el proceso puede crear el ejecutor de D-Bus"),
    );
    gpui_platform::application()
        .with_assets(Recursos(recursos))
        .run(move |cx: &mut App| {
            cx.bind_keys(entrada::atajos());
            let al_cerrar = runtime.clone();
            cx.on_app_quit(move |_| {
                let rt = al_cerrar.clone();
                async move {
                    if escritorio::captura_activa() {
                        match rt.spawn(escritorio::cancelar_captura()).await {
                            Ok(Ok(())) => {}
                            resultado => {
                                eprintln!("No se pudo cancelar la captura al cerrar: {resultado:?}")
                            }
                        }
                    }
                }
            })
            .detach();
            let opciones = ventana::opciones(
                APP_ID,
                size(px(1240.), px(740.)),
                size(px(1080.), px(620.)),
                cx,
            );
            let ventana = cx
                .open_window(opciones, |w, cx| {
                    cx.new(|cx| Vista::new(w, cx, runtime.clone()))
                })
                .expect("el compositor acepta la ventana");
            #[cfg(feature = "pruebas-ui")]
            pruebas_ui::escuchar(ventana, cx);
            #[cfg(not(feature = "pruebas-ui"))]
            let _ = ventana;
            cx.on_window_closed(|cx, _| cx.quit()).detach();
            cx.activate(true);
        });
}

#[cfg(test)]
mod pruebas {
    use super::*;
    #[test]
    fn la_confirmacion_admite_la_normalizacion_del_compositor() {
        let a = vec![Remapeo {
            origen: "evdev:29".into(),
            destino: "evdev:1".into(),
        }];
        let b = vec![Remapeo {
            origen: "ctrl".into(),
            destino: "esc".into(),
        }];
        assert!(equivalentes(&a, &b));
        assert!(!equivalentes(&a, &[]));
    }
}
