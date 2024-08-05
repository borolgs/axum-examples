use deno_ast::{ParseParams, SourceMapOption};
use deno_core::*;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use tokio::runtime::Handle;
use url::Url;

#[derive(thiserror::Error, Debug)]
pub enum Error {
    #[error(transparent)]
    Deno(#[from] anyhow::Error),
    #[error(transparent)]
    SerdeV8(#[from] serde_v8::Error),

    #[error(transparent)]
    Parse(#[from] deno_ast::ParseDiagnostic),
    #[error(transparent)]
    Transpile(#[from] deno_ast::TranspileError),

    #[error("unexpected")]
    Unexpected(String),
}

#[derive(Deserialize, Debug)]
pub struct Script {
    pub source: String,
    pub lang: Option<Lang>,
}

#[derive(Deserialize, Debug)]
#[serde(rename_all = "lowercase")]
pub enum Lang {
    TS,
    JS,
}

#[derive(Serialize, Debug)]
pub struct ScriptResult {
    pub result: Value,
}

enum Message {
    ExecuteScript {
        script: Script,
        respond_to: tokio::sync::oneshot::Sender<Result<ScriptResult, Error>>,
    },
}

#[derive(Clone)]
pub struct Runtime {
    sender: crossbeam_channel::Sender<Message>,
}

#[op2(fast)]
pub fn op_print(#[string] msg: &str, is_err: bool) -> Result<(), anyhow::Error> {
    print!("{msg}");
    Ok(())
}

impl Runtime {
    pub fn new() -> Self {
        let (sender, receiver) = crossbeam_channel::unbounded::<Message>();

        let handle = Handle::current();

        std::thread::spawn(move || {
            let print_ext = Extension {
                name: "override",
                middleware_fn: Some(Box::new(|op| match op.name {
                    "op_print" => op_print(),
                    _ => op,
                })),
                ..Default::default()
            };

            let mut runtime = JsRuntime::new(RuntimeOptions {
                extensions: vec![print_ext],
                ..Default::default()
            });

            // TODO
            let _ = runtime.execute_script("", "delete Deno");

            while let Ok(msg) = receiver.recv() {
                match msg {
                    Message::ExecuteScript { script, respond_to } => {
                        let specifier = Url::parse("test:test/test.ts").unwrap();

                        let source = if script
                            .lang
                            .map(|lang| matches!(lang, Lang::TS))
                            .unwrap_or(false)
                        {
                            match transpile_ts(specifier.clone(), script.source) {
                                Ok(source) => source,
                                Err(err) => {
                                    let _ = respond_to.send(Err(err));
                                    continue;
                                }
                            }
                        } else {
                            script.source
                        };

                        let future = async {
                            let res = async {
                                let mod_id = runtime
                                    .load_side_es_module_from_code(&specifier, source)
                                    .await?;
                                let mod_res = runtime.mod_evaluate(mod_id);
                                runtime.run_event_loop(Default::default()).await?;

                                mod_res.await?;

                                match runtime.get_module_namespace(mod_id) {
                                    Ok(global) => {
                                        let scope = &mut runtime.handle_scope();
                                        let local = v8::Local::new(scope, global);

                                        let deserialized_value = serde_v8::from_v8::<
                                            serde_json::Value,
                                        >(
                                            scope, local.into()
                                        );

                                        deserialized_value
                                            .map_err(|e| Error::from(e))
                                            .map(|result| ScriptResult { result })
                                    }
                                    Err(e) => Err(e.into()),
                                }
                            }
                            .await;

                            let _ = respond_to.send(res);
                        };

                        _ = handle.block_on(future);
                    }
                }
            }
        });

        Self { sender }
    }

    pub async fn execute_script(&self, script: Script) -> Result<ScriptResult, Error> {
        let (sender, receiver) = tokio::sync::oneshot::channel();
        let msg = Message::ExecuteScript {
            script,
            respond_to: sender,
        };

        let _ = self.sender.send(msg);
        let res = receiver
            .await
            .map_err(|e| Error::Unexpected(e.to_string()))?;

        res
    }
}

fn transpile_ts(specifier: Url, source: String) -> Result<String, Error> {
    let parsed = deno_ast::parse_module(ParseParams {
        specifier,
        text: source.into(),
        media_type: deno_ast::MediaType::TypeScript,
        capture_tokens: false,
        scope_analysis: false,
        maybe_syntax: None,
    })?;

    let res = parsed.transpile(
        &deno_ast::TranspileOptions {
            imports_not_used_as_values: deno_ast::ImportsNotUsedAsValues::Remove,
            use_decorators_proposal: true,
            ..Default::default()
        },
        &deno_ast::EmitOptions {
            source_map: SourceMapOption::Separate,
            inline_sources: true,
            ..Default::default()
        },
    )?;
    let res = res.into_source();

    Ok(String::from_utf8(res.source).unwrap())
}
