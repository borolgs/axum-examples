# Execute JS (quickjs-rusty)

```bash
cargo run --bin js-quickjs-rusty
cargo watch -q -c -x "run --bin js-quickjs-rusty" -w js-quickjs-rusty/src
```

```bash
brew install hurl
brew install jq

hurl js-quickjs-rusty/api.hurl | jq
```
