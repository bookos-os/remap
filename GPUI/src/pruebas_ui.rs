//! Solo pruebas de la ventana propia; no inyecta entrada global en la sesión.
use crate::{Servicio, Vista};
use gpui::{
    AnyWindowHandle, App, KeyUpEvent, Keystroke, MouseButton, MouseDownEvent, MouseMoveEvent,
    MouseUpEvent, PlatformInput, ScrollDelta, ScrollWheelEvent, TouchPhase, WindowHandle, point,
    px, size,
};
use serde_json::{Value, json};
use std::{path::PathBuf, time::Duration};

pub fn escuchar(ventana: WindowHandle<Vista>, cx: &App) {
    let Some(ruta) = std::env::var_os("BOOKOS_TECLAS_PRUEBAS").map(PathBuf::from) else {
        return;
    };
    let recibo = ruta.with_extension("respuesta");
    let cualquiera: AnyWindowHandle = ventana.into();
    cx.spawn(async move |cx| {
        loop {
            cx.background_executor().timer(Duration::from_millis(50)).await;
            let texto = match std::fs::read_to_string(&ruta) {
                Ok(s) => s,
                Err(e) if e.kind() == std::io::ErrorKind::NotFound => continue,
                Err(e) => {eprintln!("No se pudo leer la orden: {e}");break;}
            };
            if let Err(e) = std::fs::remove_file(&ruta) {eprintln!("No se pudo retirar la orden: {e}");break;}
            let orden: Value = match serde_json::from_str(&texto) {
                Ok(o) => o,
                Err(e) => {eprintln!("Orden inválida: {e}");break;}
            };
            let accion = orden["accion"].as_str().unwrap_or("");
            let resultado = if matches!(accion, "estado" | "tema" | "texto") {
                ventana.update(cx, |v, w, cx| {
                    match accion {
                        "estado" => return json!({
                            "seleccion": v.seleccion, "distribucion": v.distribucion,
                            "remapeos": v.remapeos, "error": v.error, "guardando": v.guardando,
                            "dialogo": match v.dialogo {None=>"ninguno",Some(crate::Dialogo::Captura(_))=>"captura",Some(_)=>"quitar"},
                            "pulsadas": v.pulsadas, "idioma": v.idioma.t("app.title"),
                            "servicio": match v.servicio {Servicio::Cargando=>"cargando",Servicio::Ausente=>"ausente",Servicio::SinLectura=>"sin_lectura",Servicio::Disponible(crate::Motor::Sistema)=>"sistema",_=>"escritorio"},
                            "tamano": [f32::from(w.viewport_size().width), f32::from(w.viewport_size().height)]
                        }),
                        "tema" => {v.tema=if orden["oscuro"].as_bool()==Some(true){bookos_ui::Tema::oscuro()}else{bookos_ui::Tema::claro()};cx.notify();}
                        "texto" => if let Some(e)=&v.comando {e.update(cx,|e,cx|e.poner_texto(orden["texto"].as_str().unwrap_or("").to_owned(),cx));},
                        _ => unreachable!("solo se leen o editan datos en esta rama"),
                    }
                    json!({"correcto":true})
                })
            } else {
                // Despachar sin prestar Vista: el evento vuelve a entrar por sus listeners.
                cualquiera.update(cx, |_, w, cx| {
                    let n=|k:&str| orden[k].as_f64().unwrap_or(0.) as f32;
                    match accion {
                        "activar" => w.activate_window(),
                        "tecla" => {
                            let mut k=Keystroke::parse(orden["tecla"].as_str().unwrap_or("escape")).expect("tecla válida de la prueba");
                            k.native_key_code=orden["codigo"].as_u64().map(|n|n as u16);
                            w.dispatch_keystroke(k.clone(),cx);
                            if orden["mantener"].as_bool()!=Some(true) {w.dispatch_event(PlatformInput::KeyUp(KeyUpEvent{keystroke:k}),cx);}
                        }
                        "soltar" => {
                            let mut k=Keystroke::parse("a").expect("tecla válida");
                            k.native_key_code=Some(n("codigo") as u16);
                            w.dispatch_event(PlatformInput::KeyUp(KeyUpEvent{keystroke:k}),cx);
                        }
                        "clic" => {
                            let position=point(px(n("x")),px(n("y")));
                            w.dispatch_event(PlatformInput::MouseMove(MouseMoveEvent{position,pressed_button:None,modifiers:Default::default()}),cx);
                            w.dispatch_event(PlatformInput::MouseDown(MouseDownEvent{position,button:MouseButton::Left,click_count:1,..Default::default()}),cx);
                            w.dispatch_event(PlatformInput::MouseUp(MouseUpEvent{position,button:MouseButton::Left,click_count:1,..Default::default()}),cx);
                        }
                        "mover" => {w.dispatch_event(PlatformInput::MouseMove(MouseMoveEvent{position:point(px(n("x")),px(n("y"))),pressed_button:None,modifiers:Default::default()}),cx);}
                        "rueda" => {w.dispatch_event(PlatformInput::ScrollWheel(ScrollWheelEvent{position:point(px(n("x")),px(n("y"))),delta:ScrollDelta::Pixels(point(px(n("dx")),px(n("dy")))),modifiers:Default::default(),touch_phase:TouchPhase::Moved}),cx);}
                        "tamano" => w.resize(size(px(n("ancho")),px(n("alto")))),
                        "cerrar" => w.remove_window(),
                        _ => return json!({"error":"Orden desconocida"}),
                    }
                    json!({"correcto":true})
                })
            }.unwrap_or_else(|e|json!({"error":e.to_string()}));
            if let Err(e)=std::fs::write(&recibo,resultado.to_string()) {eprintln!("No se pudo guardar la respuesta: {e}");break;}
        }
    }).detach();
}
