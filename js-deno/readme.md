# Execute JS (deno_core)

[js-deno/src/main.rs](./src/main.rs)

```bash
run --bin js-deno
cargo watch -q -c -x "run --bin js-deno" -w js-deno
```

[Try it out](./api.http) using the [VSCode REST Client](https://marketplace.visualstudio.com/items?itemName=humao.rest-client) or `hurl`:

```bash
brew install hurl
brew install jq

hurl js-deno/api.hurl | jq
```

