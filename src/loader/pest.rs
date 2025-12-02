use anyhow::Context;
use bevy::asset::AssetLoader;
use pest::Parser;
use thiserror::Error;

use crate::{Act, compiler::ast::{Rule, SabiParser, build_scenes}};

#[derive(Debug, Error)]
pub(crate) enum PestLoaderError {
    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),
    #[error("Conversion error: {0}")]
    Conversion(#[from] std::string::FromUtf8Error),
    #[error("Parsing error: {0}")]
    Parse(#[from] pest::error::Error<Rule>),
    #[error("Syntax error: {0}")]
    Syntax(#[from] anyhow::Error)
}

#[derive(Default)]
pub(crate) struct PestLoader;
impl AssetLoader for PestLoader {
    type Asset = Act;
    type Settings = ();
    type Error = PestLoaderError;

    fn load(
        &self,
        reader: &mut dyn bevy::asset::io::Reader,
        _settings: &Self::Settings,
        load_context: &mut bevy::asset::LoadContext,
    ) -> impl bevy::tasks::ConditionalSendFuture<Output = std::result::Result<Self::Asset, Self::Error>> {
        
        Box::pin(async move {
            let mut bytes = Vec::new();
            reader.read_to_end(&mut bytes).await?;
            let script_contents = String::from_utf8(bytes)?;
            let scene_pair = SabiParser::parse(Rule::act, &script_contents)?.next().context("Script file is empty")?;
            let mut act = build_scenes(scene_pair)?;
            let path = load_context.asset_path().path();
            let file_name = path.file_stem().and_then(|n| n.to_str()).unwrap_or("");
            act.name = file_name.into();
            Ok(act)
        })
    }
    
    fn extensions(&self) -> &[&str] {
        &["sabi"]
    }
}
