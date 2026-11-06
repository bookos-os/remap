use serde::{Deserialize, Serialize};

pub const NOMBRES: &[(u32, &str)] = &[
    (29, "ctrl"),
    (97, "ctrl_der"),
    (42, "shift"),
    (54, "shift_der"),
    (56, "alt"),
    (100, "altgr"),
    (125, "meta"),
    (126, "meta_der"),
    (1, "esc"),
    (14, "retroceso"),
    (15, "tab"),
    (28, "intro"),
    (57, "espacio"),
    (58, "bloqmayus"),
    (99, "impr"),
    (102, "inicio"),
    (103, "arriba"),
    (104, "repag"),
    (105, "izquierda"),
    (106, "derecha"),
    (107, "fin"),
    (108, "abajo"),
    (109, "avpag"),
    (110, "insert"),
    (111, "supr"),
    (113, "silencio"),
    (114, "bajar_volumen"),
    (115, "subir_volumen"),
    (127, "menu"),
    (163, "siguiente"),
    (164, "reproducir"),
    (165, "anterior"),
];
pub const DISTRIBUCIONES: &[(&str, &str)] = &[
    ("es", "Español"),
    ("pt", "Português"),
    ("fr", "Français"),
    ("de", "Deutsch"),
    ("it", "Italiano"),
    ("gb", "English (UK)"),
    ("us", "English (US)"),
];
pub const GRUPOS: &[(&str, &[&str])] = &[
    (
        "grp.mods",
        &["ctrl", "ctrl_der", "shift", "alt", "altgr", "meta"],
    ),
    (
        "grp.keys",
        &[
            "esc", "supr", "insert", "inicio", "fin", "repag", "avpag", "menu", "impr",
        ],
    ),
    (
        "grp.media",
        &[
            "reproducir",
            "anterior",
            "siguiente",
            "silencio",
            "subir_volumen",
            "bajar_volumen",
        ],
    ),
    (
        "grp.bookos",
        &[
            "accion:actividades",
            "accion:buscador",
            "accion:launchpad",
            "accion:vista_escritorios",
            "accion:mostrar_escritorio",
            "accion:captura",
            "accion:bloquear",
            "accion:terminal",
        ],
    ),
    ("grp.other", &["cmd:", "nada"]),
];

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct Remapeo {
    pub origen: String,
    pub destino: String,
}

pub fn origen(codigo: u32) -> String {
    NOMBRES
        .iter()
        .find(|(c, _)| *c == codigo)
        .map_or_else(|| format!("evdev:{codigo}"), |(_, n)| (*n).to_owned())
}

pub fn codigo(nombre: &str) -> Option<u32> {
    nombre
        .strip_prefix("evdev:")
        .and_then(|s| s.parse().ok())
        .or_else(|| NOMBRES.iter().find(|(_, n)| *n == nombre).map(|(c, _)| *c))
}

pub fn distribucion_xkb(texto: &str) -> Option<&'static str> {
    let base = texto.split(['(', ',']).next()?.trim().to_lowercase();
    let base = match base.as_str() {
        "latam" => "es",
        "uk" => "gb",
        v => v,
    };
    DISTRIBUCIONES
        .iter()
        .find(|(id, _)| *id == base)
        .map(|(id, _)| *id)
}

pub fn categoria(destino: &str) -> &'static str {
    if destino.starts_with("accion:") {
        "bookos"
    } else if destino.starts_with("cmd:") {
        "cmd"
    } else if destino == "nada" {
        "nada"
    } else {
        "tecla"
    }
}

#[derive(Clone, Debug)]
pub struct Tecla {
    pub codigo: Option<u32>,
    pub etiqueta: String,
    pub columna: f32,
    pub ancho: f32,
    pub y: f32,
    pub alto: f32,
    pub intro_iso: bool,
}

pub fn construir(iso: bool) -> Vec<Tecla> {
    let mut teclas = Vec::new();
    let mut fila = |numero: usize, celdas: Vec<(Option<u32>, String, f32)>| {
        let mut columna = 0.;
        for (codigo, etiqueta, ancho) in celdas {
            teclas.push(Tecla {
                codigo,
                etiqueta,
                columna,
                ancho: ancho * 4.,
                y: if numero == 0 {
                    0.
                } else {
                    40. + (numero - 1) as f32 * 54.
                },
                alto: if numero == 0 { 34. } else { 48. },
                intro_iso: false,
            });
            columna += ancho * 4.;
        }
    };
    let k = |c, l: &str, w| (Some(c), l.to_owned(), w);
    let caracteres = |desde, cuantos| {
        (desde..desde + cuantos)
            .map(|c| k(c, "", 1.))
            .collect::<Vec<_>>()
    };
    let mut f = vec![k(1, "lbl.esc", 1.)];
    f.extend(
        [59, 60, 61, 62, 63, 64, 65, 66, 67, 68, 87, 88]
            .into_iter()
            .enumerate()
            .map(|(i, c)| (Some(c), format!("F{}", i + 1), 1.)),
    );
    f.push(k(111, "lbl.supr", 2.));
    fila(0, f);
    let mut f = vec![k(41, "", 1.)];
    f.extend(caracteres(2, 12));
    f.push(k(14, "lbl.retroceso", 2.));
    fila(1, f);
    let mut f = vec![k(15, "lbl.tab", 1.5)];
    f.extend(caracteres(16, 12));
    if !iso {
        f.push(k(43, "", 1.5));
    }
    fila(2, f);
    let mut f = vec![k(58, "lbl.bloqmayus", 1.75)];
    f.extend(caracteres(30, 11));
    f.push(if iso {
        k(43, "", 1.)
    } else {
        k(28, "lbl.intro", 2.25)
    });
    fila(3, f);
    let mut f = vec![k(42, "lbl.shift", if iso { 1.25 } else { 2.25 })];
    if iso {
        f.push(k(86, "", 1.));
    }
    f.extend(caracteres(44, 10));
    f.push(k(54, "lbl.shift", 2.75));
    fila(4, f);
    fila(
        5,
        vec![
            k(29, "lbl.ctrl", 1.25),
            (None, "lbl.fn".into(), 1.),
            k(125, "lbl.meta", 1.25),
            k(56, "lbl.alt", 1.25),
            k(57, " ", 3.5),
            k(100, "lbl.altgr", 1.25),
            k(193, "lbl.copilot", 1.25),
            k(97, "lbl.ctrl", 1.25),
            k(105, "arrow-left", 1.),
            k(103, "arrow-up", 1.),
            k(106, "arrow-right", 1.),
        ],
    );
    let arriba = teclas
        .iter_mut()
        .find(|t| t.codigo == Some(103))
        .expect("fila de flechas incluida");
    arriba.alto = 22.;
    let mut abajo = arriba.clone();
    abajo.codigo = Some(108);
    abajo.etiqueta = "arrow-down".into();
    abajo.y += 26.;
    teclas.push(abajo);
    if iso {
        teclas.push(Tecla {
            codigo: Some(28),
            etiqueta: "lbl.intro".into(),
            columna: 55.,
            ancho: 5.,
            y: 94.,
            alto: 102.,
            intro_iso: true,
        });
    }
    teclas
}

#[cfg(test)]
mod pruebas {
    use super::*;
    #[test]
    fn iso_y_ansi_conservan_sus_teclas_fisicas() {
        for iso in [false, true] {
            let t = construir(iso);
            assert_eq!(t.iter().filter(|t| t.codigo == Some(28)).count(), 1);
            assert_eq!(t.iter().any(|t| t.codigo == Some(86)), iso);
            assert!(t.iter().all(|t| t.columna + t.ancho <= 60.));
            let mut codigos: Vec<_> = t.iter().filter_map(|t| t.codigo).collect();
            let cantidad = codigos.len();
            codigos.sort();
            codigos.dedup();
            assert_eq!(codigos.len(), cantidad);
        }
    }
    #[test]
    fn conserva_los_nombres_del_protocolo_y_las_teclas_no_dibujadas() {
        for &(c, n) in NOMBRES {
            assert_eq!(codigo(&origen(c)), Some(c));
            assert_eq!(origen(c), n);
        }
        assert_eq!(origen(240), "evdev:240");
        assert_eq!(codigo("evdev:240"), Some(240));
        assert_eq!(origen(193), "evdev:193");
        assert_eq!(codigo("copilot"), None);
        assert_eq!(distribucion_xkb("es(cat)"), Some("es"));
        assert_eq!(distribucion_xkb("latam,us"), Some("es"));
    }
}
