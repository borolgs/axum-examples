use quickjs_rusty::{
    Context, ExecutionError, OwnedJsValue,
    console::{ConsoleBackend, Level},
    serde::to_js,
};

use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use std::fmt::Write;
use std::sync::{Arc, Mutex};

#[derive(thiserror::Error, Debug)]
pub enum Error {
    #[error(transparent)]
    Execution(#[from] ExecutionError),
    #[error(transparent)]
    Serde(#[from] quickjs_rusty::serde::Error),
    #[error("unexpected")]
    Unexpected(String),
}

#[derive(Deserialize, Debug)]
pub struct Script {
    pub args: Option<Value>,
    pub source: String,
}

#[derive(Serialize, Debug)]
pub struct ScriptResult {
    pub result: String,
    pub console_output: String,
}

enum Message {
    ExecuteScript {
        script: Script,
        respond_to: tokio::sync::oneshot::Sender<Result<ScriptResult, Error>>,
    },
}

#[derive(Clone)]
pub struct Runtime {
    sender: std::sync::mpsc::Sender<Message>,
}

impl Runtime {
    pub fn new() -> Self {
        let (sender, receiver) = std::sync::mpsc::channel::<Message>();

        std::thread::spawn(move || {
            let context = Context::builder().build().unwrap();

            let js_context = unsafe { context.context_raw() };

            let ctx = to_js(js_context, &json!({"name": "script"})).unwrap();

            context.set_global("ctx", ctx).unwrap();

            while let Ok(msg) = receiver.recv() {
                match msg {
                    Message::ExecuteScript { script, respond_to } => {
                        _ = respond_to.send(Runtime::eval(script, &context));
                    }
                };
            }
        });

        Self { sender }
    }

    fn eval(script: Script, context: &Context) -> Result<ScriptResult, Error> {
        let console = Console::new();
        let output = console.output.clone();

        context.set_console(Box::new(console))?;

        let js_context = unsafe { context.context_raw() };
        let args = to_js(js_context, &script.args)?;
        context.set_global("args", args)?;

        let result = context.eval(&script.source, false)?;
        let result = result.js_to_string()?;

        let output = output.lock().unwrap();
        let console_output = output.clone();

        Ok(ScriptResult {
            result,
            console_output,
        })
    }

    pub async fn execute_script(&self, script: Script) -> Result<ScriptResult, Error> {
        let (sender, receiver) = tokio::sync::oneshot::channel::<Result<ScriptResult, Error>>();

        let msg = Message::ExecuteScript {
            script,
            respond_to: sender,
        };

        _ = self.sender.send(msg);

        let res = receiver
            .await
            .map_err(|e| Error::Unexpected(e.to_string()))?;

        res
    }
}

struct Console {
    output: Arc<Mutex<String>>,
}

impl Console {
    fn new() -> Self {
        Self {
            output: Arc::new(Mutex::new(String::from(""))),
        }
    }
}

impl ConsoleBackend for Console {
    fn log(&self, _level: Level, values: Vec<OwnedJsValue>) {
        let output_line = values
            .into_iter()
            .map(|v| v.to_string().unwrap_or_default())
            .collect::<Vec<_>>()
            .join(", ");

        let mut output = self.output.lock().unwrap();
        writeln!(output, "{}", output_line).unwrap();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn sum() {
        let runtime = Runtime::new();
        let res = runtime
            .execute_script(Script {
                source: "console.log('test'); 1 + 1".into(),
                args: None,
            })
            .await
            .unwrap();

        assert_eq!(res.result, "2");
        assert_eq!(res.console_output, "test\n");

        let res = runtime
            .execute_script(Script {
                source: "console.log('test2'); 2 + 2".into(),
                args: None,
            })
            .await
            .unwrap();

        assert_eq!(res.result, "4");
        assert_eq!(res.console_output, "test2\n");
    }

    #[tokio::test]
    async fn ctx() {
        let runtime = Runtime::new();
        let res = runtime
            .execute_script(Script {
                source: "let obj = {name: ctx.name, args}; JSON.stringify(obj);".into(),
                args: Some(json!(["a", "b"])),
            })
            .await
            .unwrap();

        assert_eq!(res.result, "{\"name\":\"script\",\"args\":[\"a\",\"b\"]}");
    }

    #[test]
    fn example() {
        let console = Console::new();
        let output = console.output.clone();

        let context = Context::builder().console(console).build().unwrap();

        let value = context
            .eval("console.log('hello','world');console.log('!');1 + 2", false)
            .unwrap();
        println!("js: 1 + 1 = {:?}", value);

        let console_output = output.lock().unwrap();
        println!("{:?}", console_output);

        let context = context.reset().unwrap();

        let console = Console::new();
        let output = console.output.clone();

        _ = context.set_console(Box::new(console));

        let value = context.eval("console.log('!!!!!!');2 + 2", false).unwrap();
        println!("js: 2 + 2 = {:?}", value);

        let console_output = output.lock().unwrap();
        println!("{:?}", console_output);
    }
}
