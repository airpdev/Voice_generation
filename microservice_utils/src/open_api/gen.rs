use openapi_rs::settings::OpenApiSettings;
use openapi_rs::gen::OpenApiGenerator;
use std::io::Write;
use std::fs::File;

use anyhow::Result;

// use crate::contacts::contacts_handler::{sync_contacts_spec, get_contacts_spec};

pub type GenSpec = Box<dyn FnOnce(&str,&mut OpenApiGenerator)>;

#[derive(Debug,Clone)]
pub struct Spec<F:FnOnce(&str,&mut OpenApiGenerator)> {
    pub route: String,
    pub gen: F
}

pub fn generate_openapi_spec<F: FnOnce(&str,&mut OpenApiGenerator)>(spec_fns: Vec<Spec<F>>) -> Result<()> {
    let mut generator = OpenApiGenerator::new(&OpenApiSettings::default());

    for spec_fn in spec_fns {
        (spec_fn.gen)(&spec_fn.route,&mut generator);
    }

    let open_api = generator.into_openapi();

    let mut spec_file = File::create("./swagger-ui/openapi.json")?;

    let json = serde_json::to_string(&open_api)?;

    spec_file.write_all(json.as_bytes())?;

    Ok(())
}