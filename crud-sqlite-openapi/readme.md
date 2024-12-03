# CRUD API Example (rusqlite, aide)

Dev:

```bash
run --bin crud-sqlite-openapi

cargo watch -q -c -x "run --bin crud-sqlite-openapi" -w crud-sqlite-openapi/src
```

[http://127.0.0.1:4000/\_\_docs\_\_](http://127.0.0.1:4000/__docs__)

Live Demo:

```bash
# build
podman machine start
cross build --package=crud-sqlite-openapi --target=x86_64-unknown-linux-musl --release
```

[https://axum-crud-openapi.glitch.me/\_\_docs\_\_](https://axum-crud-openapi.glitch.me/__docs__)
