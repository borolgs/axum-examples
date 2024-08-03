use deno_core::*;
use serde::{Deserialize, Serialize};
use serde_json::Value;

#[derive(thiserror::Error, Debug)]
pub enum Error {
    #[error(transparent)]
    Deno(#[from] anyhow::Error),
    #[error(transparent)]
    SerdeV8(#[from] serde_v8::Error),
    #[error("unexpected")]
    Unexpected(String),
}

#[derive(Deserialize, Debug)]
pub struct Script {
    pub source: String,
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

        std::thread::spawn(move || {
            let my_ext = Extension {
                name: "override",
                middleware_fn: Some(Box::new(|op| match op.name {
                    "op_print" => op_print(),
                    _ => op,
                })),
                ..Default::default()
            };

            let mut runtime = JsRuntime::new(RuntimeOptions {
                extensions: vec![my_ext],
                ..Default::default()
            });

            // TODO
            let _ = runtime.execute_script("", "delete Deno");

            while let Ok(msg) = receiver.recv() {
                match msg {
                    Message::ExecuteScript { script, respond_to } => {
                        let res = runtime
                            .lazy_load_es_module_with_code("test:test/test.js", script.source);

                        let res = match res {
                            Ok(global) => {
                                let scope = &mut runtime.handle_scope();
                                let local = v8::Local::new(scope, global);

                                let deserialized_value =
                                    serde_v8::from_v8::<serde_json::Value>(scope, local);

                                match deserialized_value {
                                    Ok(value) => Ok(value),
                                    Err(err) => Err(err.into()),
                                }
                            }
                            Err(err) => Err(err.into()),
                        }
                        .map(|result| ScriptResult { result });

                        let _ = respond_to.send(res);
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
