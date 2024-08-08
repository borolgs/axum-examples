# CRUD Example (rusqlite, htmx)

Dev:

```bash
run --bin crud-sqlite-htmx

cargo watch -q -c -x "run --bin crud-sqlite-htmx" -w crud-sqlite-htmx/src
```

http://127.0.0.1:4000

Live Demo:

```bash
# build
podman machine start
cross build --package=crud-sqlite-htmx --target=x86_64-unknown-linux-musl --release
```

https://axum-crud-example.glitch.me
