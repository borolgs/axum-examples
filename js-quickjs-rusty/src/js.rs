use quickjs_rusty::{
    Context, ExecutionError, JsCompiledFunction, OwnedJsValue,
    console::{ConsoleBackend, Level},
    serde::to_js,
};

use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use std::sync::{Arc, Mutex};
use std::{collections::HashMap, fmt::Write};

#[derive(thiserror::Error, Debug)]
pub enum Error {
    #[error(transparent)]
    Execution(#[from] ExecutionError),
    #[error(transparent)]
    Serde(#[from] quickjs_rusty::serde::Error),

    #[error(transparent)]
    Parse(#[from] deno_ast::ParseDiagnostic),
    #[error(transparent)]
    Transpile(#[from] deno_ast::TranspileError),

    #[error("unexpected")]
    Unexpected(String),
}

#[derive(Deserialize, Debug)]
#[serde(untagged)]
pub enum Script {
    Function { args: Option<Value>, code: String },
    CompiledFunction { args: Option<Value>, name: String },
}

#[derive(Debug)]
enum Function {
    Code(String),
    Compiled(JsCompiledFunction),
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

            let sum_handler = include_str!("../src-ts/sum.ts");
            let sum_handler = transpile_ts(sum_handler).unwrap();

            let mut compiled_fns = HashMap::new();

            let sum_fn = quickjs_rusty::compile::compile(js_context, &sum_handler, "test.js")
                .unwrap()
                .try_into_compiled_function()
                .unwrap();

            compiled_fns.insert(String::from("sum"), sum_fn);

            while let Ok(msg) = receiver.recv() {
                match msg {
                    Message::ExecuteScript { script, respond_to } => {
                        let source = Runtime::prepare(script, &compiled_fns);

                        let msg = match source {
                            Ok((args, source)) => Runtime::eval(source, args, &context),
                            Err(err) => Err(err),
                        };

                        _ = respond_to.send(msg);
                    }
                };
            }
        });

        Self { sender }
    }

    fn prepare(
        script: Script,
        compiled_fns: &HashMap<String, JsCompiledFunction>,
    ) -> Result<(Option<Value>, Function), Error> {
        match script {
            Script::Function { args, code } => Ok((args, Function::Code(code))),
            Script::CompiledFunction { args, name } => {
                let function = compiled_fns
                    .get(&name)
                    .ok_or(Error::Unexpected(format!("function {} not found", name)))?
                    .to_owned();

                Ok((args, Function::Compiled(function)))
            }
        }
    }

    fn eval(
        source: Function,
        args: Option<Value>,
        context: &Context,
    ) -> Result<ScriptResult, Error> {
        let console = Console::new();
        let output = console.output.clone();

        context.set_console(Box::new(console))?;

        let js_context = unsafe { context.context_raw() };
        let args = to_js(js_context, &args)?;
        context.set_global("args", args)?;

        let result = match source {
            Function::Code(code) => context.eval(&code, false)?,
            Function::Compiled(compiled_fn) => compiled_fn.eval()?,
        };
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

fn transpile_ts(source: &str) -> Result<String, Error> {
    let parsed = deno_ast::parse_module(deno_ast::ParseParams {
        specifier: deno_ast::ModuleSpecifier::parse("test://script.ts").unwrap(),
        text: source.into(),
        media_type: deno_ast::MediaType::TypeScript,
        capture_tokens: false,
        scope_analysis: false,
        maybe_syntax: None,
    })?;

    let res = parsed
        .transpile(
            &deno_ast::TranspileOptions {
                imports_not_used_as_values: deno_ast::ImportsNotUsedAsValues::Remove,
                use_decorators_proposal: true,
                ..Default::default()
            },
            &deno_ast::TranspileModuleOptions {
                ..Default::default()
            },
            &deno_ast::EmitOptions {
                source_map: deno_ast::SourceMapOption::Separate,
                inline_sources: true,
                ..Default::default()
            },
        )?
        .into_source();

    Ok(res.text)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn sum() {
        let runtime = Runtime::new();
        let res = runtime
            .execute_script(Script::Function {
                code: "console.log('test'); 1 + 1".into(),
                args: None,
            })
            .await
            .unwrap();

        assert_eq!(res.result, "2");
        assert_eq!(res.console_output, "test\n");

        let res = runtime
            .execute_script(Script::Function {
                code: "console.log('test2'); 2 + 2".into(),
                args: None,
            })
            .await
            .unwrap();

        assert_eq!(res.result, "4");
        assert_eq!(res.console_output, "test2\n");
    }

    #[tokio::test]
    async fn sum_compiled() {
        let runtime = Runtime::new();
        let res = runtime
            .execute_script(Script::CompiledFunction {
                name: "sum".into(),
                args: Some(json!({"a": 1, "b": 1})),
            })
            .await
            .unwrap();

        assert_eq!(res.result, "2");
        assert_eq!(res.console_output, "a + b = 2\n");
    }

    #[tokio::test]
    async fn ctx() {
        let runtime = Runtime::new();
        let res = runtime
            .execute_script(Script::Function {
                code: "let obj = {name: ctx.name, args}; JSON.stringify(obj);".into(),
                args: Some(json!(["a", "b"])),
            })
            .await
            .unwrap();

        assert_eq!(res.result, "{\"name\":\"script\",\"args\":[\"a\",\"b\"]}");
    }

    #[test]
    fn transpile() {
        let source = "export type A = {args; any}; function a(args: A): {res: any} {};";
        assert_eq!(
            transpile_ts(source.into()).unwrap(),
            "function a(args) {}\n"
        );
    }

    #[test]
    fn compile() {
        let context = Context::builder().build().unwrap();
        let js_context = unsafe { context.context_raw() };

        let source = "args.a + args.b";

        let compiled_fn = quickjs_rusty::compile::compile(js_context, &source, "test.js")
            .unwrap()
            .try_into_compiled_function()
            .unwrap();

        let args = to_js(js_context, &json!({"a": 1, "b": 1})).unwrap();
        context.set_global("args", args).unwrap();

        let res = compiled_fn.eval().unwrap().to_int().unwrap();

        assert_eq!(res, 2);

        let args = to_js(js_context, &json!({"a": 2, "b": 2})).unwrap();
        context.set_global("args", args).unwrap();

        let res = compiled_fn.eval().unwrap().to_int().unwrap();

        assert_eq!(res, 4);
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
