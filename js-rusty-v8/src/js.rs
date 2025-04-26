use serde::{Deserialize, Serialize};

#[derive(thiserror::Error, Debug)]
pub enum Error {
    #[error("unexpected")]
    Unexpected(String),
}

#[derive(Deserialize, Debug)]
#[serde(rename_all = "lowercase")]
pub enum Lang {
    JS,
}

#[derive(Deserialize, Debug)]
pub struct Script {
    pub source: String,
    pub lang: Option<Lang>,
}

#[derive(Serialize, Debug)]
pub struct ScriptResult {
    pub result: String,
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
            // Initialize V8.
            let platform = v8::new_default_platform(0, false).make_shared();
            v8::V8::initialize_platform(platform);
            v8::V8::initialize();

            // Create a new Isolate and make it the current one.
            let isolate = &mut v8::Isolate::new(v8::CreateParams::default());

            while let Ok(msg) = receiver.recv() {
                match msg {
                    Message::ExecuteScript { script, respond_to } => {
                        // Create a stack-allocated handle scope.
                        let handle_scope = &mut v8::HandleScope::new(isolate);

                        // Create a new context.
                        let context = v8::Context::new(handle_scope, Default::default());

                        // Enter the context for compiling and running the hello world script.
                        let scope = &mut v8::ContextScope::new(handle_scope, context);

                        // Create a string containing the JavaScript source code.
                        let code = v8::String::new(scope, &script.source).unwrap();

                        // Compile the source code.
                        let script = v8::Script::compile(scope, code, None).unwrap();

                        // Run the script to get the result.
                        let result = script.run(scope).unwrap();

                        let deserialized_value =
                            result.to_string(scope).unwrap().to_rust_string_lossy(scope);

                        _ = respond_to.send(Ok(ScriptResult {
                            result: deserialized_value,
                        }));
                    }
                };
            }
        });

        Self { sender }
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

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn sum() {
        let runtime = Runtime::new();
        let res = runtime
            .execute_script(Script {
                source: "1 + 1".into(),
                lang: None,
            })
            .await
            .unwrap()
            .result;

        assert_eq!(res, "2");
    }

    #[test]
    fn example() {
        // Initialize V8.
        let platform = v8::new_default_platform(0, false).make_shared();
        v8::V8::initialize_platform(platform);
        v8::V8::initialize();

        // Create a new Isolate and make it the current one.
        let isolate = &mut v8::Isolate::new(v8::CreateParams::default());

        // Create a stack-allocated handle scope.
        let handle_scope = &mut v8::HandleScope::new(isolate);

        // Create a new context.
        let context = v8::Context::new(handle_scope, Default::default());

        // Enter the context for compiling and running the hello world script.
        let scope = &mut v8::ContextScope::new(handle_scope, context);

        // Create a string containing the JavaScript source code.
        let code = v8::String::new(scope, "'Hello' + ' World!'").unwrap();

        // Compile the source code.
        let script = v8::Script::compile(scope, code, None).unwrap();

        // Run the script to get the result.
        let result = script.run(scope).unwrap();

        // Convert the result to a string and print it.
        let result = result.to_string(scope).unwrap();
        println!("{}", result.to_rust_string_lossy(scope));
    }
}
