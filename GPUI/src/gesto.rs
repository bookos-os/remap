use gpui::{
    AnyElement, App, Bounds, Element, ElementId, GlobalElementId, InspectorElementId, IntoElement,
    LayoutId, Pixels, Window, point, px,
};

#[derive(Default, PartialEq)]
enum Eje {
    #[default]
    Indeciso,
    Vertical,
    Horizontal,
}

#[derive(Default)]
pub struct Gesto {
    x: f32,
    y: f32,
    eje: Eje,
}
impl Gesto {
    pub fn mover(&mut self, x: f32, y: f32) -> Option<f32> {
        self.x += x;
        self.y += y;
        if self.eje == Eje::Indeciso {
            if self.y.abs() >= 8. {
                self.eje = Eje::Vertical;
            } else if self.x.abs() >= 16. && self.x.abs() >= 2. * self.y.abs() {
                self.eje = Eje::Horizontal;
            }
        }
        (self.eje == Eje::Horizontal).then(|| {
            let recorrido = (self.x / 300.).max(0.);
            if recorrido <= 1. {
                recorrido
            } else {
                (1. + (recorrido - 1.).ln_1p() * 0.15).min(1.06)
            }
        })
    }
}

/// Cambia el origen de pintado e hitboxes; no recalcula el ancho del panel al deslizar.
pub struct Desplazado {
    hijo: AnyElement,
    x: f32,
}
pub fn desplazar(hijo: impl IntoElement, x: f32) -> Desplazado {
    Desplazado {
        hijo: hijo.into_any_element(),
        x,
    }
}
impl IntoElement for Desplazado {
    type Element = Self;
    fn into_element(self) -> Self {
        self
    }
}
impl Element for Desplazado {
    type RequestLayoutState = ();
    type PrepaintState = ();
    fn id(&self) -> Option<ElementId> {
        None
    }
    fn source_location(&self) -> Option<&'static std::panic::Location<'static>> {
        None
    }
    fn request_layout(
        &mut self,
        _: Option<&GlobalElementId>,
        _: Option<&InspectorElementId>,
        w: &mut Window,
        cx: &mut App,
    ) -> (LayoutId, ()) {
        (self.hijo.request_layout(w, cx), ())
    }
    fn prepaint(
        &mut self,
        _: Option<&GlobalElementId>,
        _: Option<&InspectorElementId>,
        _: Bounds<Pixels>,
        _: &mut (),
        w: &mut Window,
        cx: &mut App,
    ) {
        w.with_element_offset(point(px(self.x), px(0.)), |w| {
            self.hijo.prepaint(w, cx);
        });
    }
    fn paint(
        &mut self,
        _: Option<&GlobalElementId>,
        _: Option<&InspectorElementId>,
        _: Bounds<Pixels>,
        _: &mut (),
        _: &mut (),
        w: &mut Window,
        cx: &mut App,
    ) {
        self.hijo.paint(w, cx);
    }
}

#[cfg(test)]
mod pruebas {
    use super::*;
    #[test]
    fn el_scroll_vertical_bloquea_el_gesto_todo_el_tramo() {
        let mut g = Gesto::default();
        assert_eq!(g.mover(3., 9.), None);
        assert_eq!(g.mover(200., 0.), None);
    }
    #[test]
    fn el_gesto_conserva_el_recorrido_acumulado_y_limita_la_goma() {
        let mut g = Gesto::default();
        assert_eq!(g.mover(10., 1.), None);
        assert!((g.mover(20., 0.).expect("eje horizontal decidido") - 0.1).abs() < 0.001);
        assert_eq!(g.mover(10000., 0.), Some(1.06));
    }
}
