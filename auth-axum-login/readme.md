# Auth Example (github, axum_login)

```bash
run --bin auth-axum-login

# hot reload
cargo watch -q -c -x "run --bin auth-axum-login" -w auth-axum-login/src

# build
podman machine start
cross build --package=auth-axum-login --target=x86_64-unknown-linux-musl --release
```

.env

```bash
# github oauth
# https://docs.github.com/en/apps/oauth-apps/building-oauth-apps/authorizing-oauth-apps
GITHUB_OAUTH_REDIRECT_URL=
GITHUB_OAUTH_CLIENT_ID=
GITHUB_OAUTH_CLIENT_SECRET=
```

http://127.0.0.1:4000

Live Demo: https://axum-notes-app.glitch.me
