//! Marco de ventana con decoraciones propias: barra de título con sus tres
//! botones, esquinas redondeadas y asas para redimensionar.

use gpui::{
    AnyElement, App, Bounds, CursorStyle, Decorations, Div, FontWeight, IntoElement, MouseButton,
    Pixels, ResizeEdge, SharedString, Size, Window, WindowBackgroundAppearance, WindowBounds,
    WindowDecorations, WindowOptions, div, prelude::*, px, relative,
};

use crate::idioma::Idioma;
use bookos_ui::{FUENTE_UI, icono, tema::Tema};

/// Ancho de las asas de redimensionado.
const BORDE: Pixels = px(8.);

/// `app_id` tiene que coincidir con el nombre del `.desktop` y su
/// `StartupWMClass`: es lo que usa el dock para encontrar el icono.
pub fn opciones(app_id: &str, tam: Size<Pixels>, tam_min: Size<Pixels>, cx: &App) -> WindowOptions {
    WindowOptions {
        window_bounds: Some(WindowBounds::Windowed(Bounds::centered(None, tam, cx))),
        app_id: Some(app_id.into()),
        window_decorations: Some(WindowDecorations::Client),
        // Transparente para que se vean las esquinas redondeadas del marco.
        window_background: WindowBackgroundAppearance::Transparent,
        window_min_size: Some(tam_min),
        ..Default::default()
    }
}

/// La raíz de la ventana. `contenido` va bajo la barra de título y `capas`
/// (menú, toast, diálogos) encima de todo, cubriendo la ventana entera. Las
/// asas de redimensionar se pintan las últimas para que ninguna capa les
/// quite los 8 px del borde.
pub fn marco(
    tm: &Tema,
    window: &Window,
    titulo: Option<SharedString>,
    izquierda: Option<AnyElement>,
    contenido: impl IntoElement,
    capas: impl IntoIterator<Item = AnyElement>,
) -> Div {
    let dec = window.window_decorations();
    let propias = matches!(dec, Decorations::Client { .. }) && !window.is_maximized();
    div()
        .size_full()
        .relative()
        .flex()
        .flex_col()
        .font_family(FUENTE_UI)
        .bg(tm.win)
        .text_color(tm.tx)
        .when(propias, |d| {
            d.rounded(px(20.))
                .border_1()
                .border_color(tm.brd)
                .overflow_hidden()
        })
        .child(barra_titulo(tm, window, titulo, izquierda))
        .child(div().flex_1().min_h_0().child(contenido))
        .children(capas)
        .when(propias, |d| d.child(asas()))
}

fn barra_titulo(
    tm: &Tema,
    window: &Window,
    titulo: Option<SharedString>,
    izquierda: Option<AnyElement>,
) -> impl IntoElement {
    let maximizada = window.is_maximized();
    div()
        .id("barra-titulo")
        .relative()
        .flex_shrink_0()
        .h(px(36.))
        .px(px(6.))
        .flex()
        .items_center()
        .justify_between()
        .on_mouse_down(MouseButton::Left, |ev, window, _| {
            if ev.click_count == 2 {
                window.zoom_window();
            } else {
                window.start_window_move();
            }
        })
        .children(titulo.map(|t| {
            div()
                .absolute()
                .inset_0()
                .flex()
                .items_center()
                .justify_center()
                .text_size(px(13.))
                .font_weight(FontWeight::SEMIBOLD)
                .text_color(tm.tx2)
                .child(div().max_w(relative(0.6)).truncate().child(t))
        }))
        // Un hueco vacío si no hay nada, para que los botones sigan a la derecha.
        .child(
            div()
                .on_mouse_down(MouseButton::Left, |_, _, cx| cx.stop_propagation())
                .children(izquierda),
        )
        .child(
            div()
                .flex()
                .items_center()
                .gap(px(4.))
                // Que pulsar un botón no empiece a arrastrar la ventana.
                .on_mouse_down(MouseButton::Left, |_, _, cx| cx.stop_propagation())
                .child(boton(
                    tm,
                    "min",
                    "ventana-minimizar",
                    false,
                    window.is_window_active(),
                    |w, _| w.minimize_window(),
                ))
                .child(boton(
                    tm,
                    "max",
                    if maximizada {
                        "ventana-restaurar"
                    } else {
                        "ventana-maximizar"
                    },
                    false,
                    window.is_window_active(),
                    |w, _| w.zoom_window(),
                ))
                .child(boton(
                    tm,
                    "cerrar",
                    "ventana-cerrar",
                    true,
                    window.is_window_active(),
                    |w, _| w.remove_window(),
                )),
        )
}

fn boton(
    tm: &Tema,
    id: &'static str,
    svg: &'static str,
    es_cerrar: bool,
    activa: bool,
    accion: impl Fn(&mut Window, &mut App) + 'static,
) -> AnyElement {
    // El marco compartido usa 32 px; Remap y el HIG exigen 28 y un hueco de 4.
    let bg_hover = if es_cerrar {
        tm.red
    } else {
        tm.tx.opacity(if tm.oscuro { 0.17 } else { 0.14 })
    };
    let fg_hover = if es_cerrar { gpui::white() } else { tm.tx };
    let grupo = SharedString::from(format!("wm-{id}"));
    let etiqueta = Idioma::detectar().t(match id {
        "min" => "wm.minimize",
        "max" => "wm.maximize",
        _ => "wm.close",
    });
    let tema_actual = *tm;
    div()
        .id(id)
        .group(grupo.clone())
        .tab_index(0)
        .focus_visible(move |s| s.bg(bg_hover))
        .tooltip(move |_, cx| {
            cx.new(|_| crate::Ayuda {
                tema: tema_actual,
                lineas: vec![etiqueta.clone()],
            })
            .into()
        })
        .w(px(28.))
        .h(px(28.))
        .rounded(px(6.))
        .flex()
        .items_center()
        .justify_center()
        .cursor_pointer()
        .opacity(if activa { 0.55 } else { 0.30 })
        .hover(move |s| s.bg(bg_hover).opacity(1.))
        .on_click(move |_, window, cx| accion(window, cx))
        .child(icono(svg, 16., tm.tx).group_hover(grupo, move |s| s.text_color(fg_hover)))
        .into_any_element()
}

fn asas() -> impl IntoElement {
    let asa = |borde: ResizeEdge, cursor: CursorStyle| {
        let d = div()
            .on_mouse_down(MouseButton::Left, move |_, window, cx| {
                cx.stop_propagation();
                window.start_window_resize(borde);
            })
            .cursor(cursor)
            .absolute();
        match borde {
            ResizeEdge::Top => d.top_0().left(BORDE).right(BORDE).h(BORDE),
            ResizeEdge::Bottom => d.bottom_0().left(BORDE).right(BORDE).h(BORDE),
            ResizeEdge::Left => d.left_0().top(BORDE).bottom(BORDE).w(BORDE),
            ResizeEdge::Right => d.right_0().top(BORDE).bottom(BORDE).w(BORDE),
            ResizeEdge::TopLeft => d.top_0().left_0().size(BORDE),
            ResizeEdge::TopRight => d.top_0().right_0().size(BORDE),
            ResizeEdge::BottomLeft => d.bottom_0().left_0().size(BORDE),
            ResizeEdge::BottomRight => d.bottom_0().right_0().size(BORDE),
        }
    };
    // Cubre la ventana pero sin escuchar nada: los clics solo los paran las
    // asas, el resto pasa al contenido.
    div()
        .absolute()
        .inset_0()
        .child(asa(ResizeEdge::Top, CursorStyle::ResizeUpDown))
        .child(asa(ResizeEdge::Bottom, CursorStyle::ResizeUpDown))
        .child(asa(ResizeEdge::Left, CursorStyle::ResizeLeftRight))
        .child(asa(ResizeEdge::Right, CursorStyle::ResizeLeftRight))
        .child(asa(ResizeEdge::TopLeft, CursorStyle::ResizeUpLeftDownRight))
        .child(asa(
            ResizeEdge::BottomRight,
            CursorStyle::ResizeUpLeftDownRight,
        ))
        .child(asa(
            ResizeEdge::TopRight,
            CursorStyle::ResizeUpRightDownLeft,
        ))
        .child(asa(
            ResizeEdge::BottomLeft,
            CursorStyle::ResizeUpRightDownLeft,
        ))
}
