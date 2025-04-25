# Execute JS (rusty_v8)

```bash
run --bin js-rusty-v8
cargo watch -q -c -x "run --bin js-rusty-v8" -w js-rusty-v8
```

```bash
brew install hurl
brew install jq

hurl js-rusty-v8/api.hurl | jq
```
