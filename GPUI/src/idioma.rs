use std::{collections::HashMap, sync::LazyLock};

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Idioma {
    Espanol,
    Ingles,
}

static ES: LazyLock<HashMap<String, String>> = LazyLock::new(|| {
    serde_json::from_str(include_str!("es.json"))
        .expect("catálogo español válido incluido en el binario")
});
static EN: LazyLock<HashMap<String, String>> = LazyLock::new(|| {
    serde_json::from_str(include_str!("en.json"))
        .expect("catálogo inglés válido incluido en el binario")
});

impl Idioma {
    pub fn detectar() -> Self {
        Self::de_variables(|clave| std::env::var(clave).ok())
    }

    fn de_variables(leer: impl Fn(&str) -> Option<String>) -> Self {
        let locale = ["LC_ALL", "LC_MESSAGES", "LANG"]
            .into_iter()
            .find_map(|k| leer(k).filter(|s| !s.is_empty()))
            .unwrap_or_default();
        if locale
            .split(['_', '-', '.', '@'])
            .next()
            .is_some_and(|s| s.eq_ignore_ascii_case("en"))
        {
            Self::Ingles
        } else {
            Self::Espanol
        }
    }

    pub fn t(self, clave: &str) -> String {
        let catalogo = if self == Self::Ingles { &*EN } else { &*ES };
        catalogo
            .get(clave)
            .or_else(|| ES.get(clave))
            .cloned()
            .unwrap_or_else(|| clave.to_owned())
    }

    pub fn con(self, clave: &str, variable: &str, valor: &str) -> String {
        self.t(clave).replace(&format!("{{{variable}}}"), valor)
    }
}

#[cfg(test)]
mod pruebas {
    use super::*;

    #[test]
    fn respeta_la_prioridad_regional_sin_cambiar_el_entorno() {
        let detectar = |vars: &[(&str, &str)]| {
            Idioma::de_variables(|k| {
                vars.iter()
                    .find(|(v, _)| *v == k)
                    .map(|(_, v)| (*v).to_owned())
            })
        };
        assert_eq!(detectar(&[("LANG", "en_US.UTF-8")]), Idioma::Ingles);
        assert_eq!(
            detectar(&[("LC_ALL", "es_ES.UTF-8"), ("LANG", "en_GB")]),
            Idioma::Espanol
        );
        assert_eq!(
            detectar(&[("LC_ALL", ""), ("LC_MESSAGES", "en-GB"), ("LANG", "es_ES")]),
            Idioma::Ingles
        );
        assert_eq!(detectar(&[("LANG", "fr_FR")]), Idioma::Espanol);
        assert_eq!(detectar(&[]), Idioma::Espanol);
    }

    #[test]
    fn los_catalogos_cubren_las_mismas_claves_y_variables() {
        assert_eq!(ES.len(), EN.len());
        for (clave, texto) in ES.iter() {
            let ingles = EN.get(clave).expect("cada texto español tiene traducción");
            for parte in texto.split('{').skip(1) {
                let variable = parte.split('}').next().expect("variable del catálogo");
                assert!(ingles.contains(&format!("{{{variable}}}")), "{clave}");
            }
        }
    }
}
