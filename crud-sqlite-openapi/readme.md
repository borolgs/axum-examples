# CRUD API Example (rusqlite, aide)

```rs
pub fn router(state: AppState) -> ApiRouter {
    ApiRouter::new()
        .api_route(
            "/api/v1/notes",
            get(find_notes).post_with(create_note, |t| t.response::<201, Json<Note>>()),
        )
        .api_route(
            "/api/v1/notes/:note_id",
            get(get_note).patch(update_note).delete(delete_note),
        )
        .with_state(state)
}

async fn find_notes(NoApi(base): NoApi<BaseParams>) -> impl IntoApiResponse {
    handlers::find_notes(base).await.map(Json)
}

async fn create_note(NoApi(base): NoApi<BaseParams>, Json(args): Json<CreateNote>) -> impl IntoApiResponse {
    handlers::create_note(args, base)
        .await
        .map(|r| (StatusCode::CREATED, Json(r)))
}

...
```

This code produces the following OpenAPI schema:

<details>
<summary>openapi: 3.1.0</summary>
<br>

```yaml
openapi: 3.1.0
info:
  title: Notes
  version: ''
paths:
  /api/v1/notes:
    get:
      responses:
        '200':
          description: ''
          content:
            application/json:
              schema:
                $ref: '#/components/schemas/FindNotesResponse'
        default:
          description: ''
          content:
            application/json:
              schema:
                $ref: '#/components/schemas/ErrorResponse'
    post:
      requestBody:
        content:
          application/json:
            schema:
              $ref: '#/components/schemas/CreateNote'
        required: true
      responses:
        '201':
          description: ''
          content:
            application/json:
              schema:
                $ref: '#/components/schemas/Note'
        default:
          description: ''
          content:
            application/json:
              schema:
                $ref: '#/components/schemas/ErrorResponse'
    ...

components:
  schemas:
    CreateNote:
      type: object
      required:
        - text
        - title
      properties:
        text:
          type: string
        title:
          type: string
    ErrorResponse:
      oneOf:
        - title: ErrorResponse
          type: object
          required:
            - error
            - status
          properties:
            details:
              type:
                - object
                - 'null'
              additionalProperties: true
            error:
              type: string
              enum:
                - not_found
            message:
              type:
                - string
                - 'null'
            status:
              type: integer
              format: uint16
              enum:
                - 404
              minimum: 0
        - title: ErrorResponse
          type: object
          required:
            - error
            - status
          properties:
            details:
              type:
                - object
                - 'null'
              additionalProperties: true
            error:
              type: string
              enum:
                - path_validation
            message:
              type:
                - string
                - 'null'
            status:
              type: integer
              format: uint16
              enum:
                - 400
              minimum: 0
        - title: ErrorResponse
          type: object
          required:
            - error
            - status
          properties:
            details:
              type:
                - object
                - 'null'
              additionalProperties: true
            error:
              type: string
              enum:
                - query_validation
            message:
              type:
                - string
                - 'null'
            status:
              type: integer
              format: uint16
              enum:
                - 400
              minimum: 0
        - title: ErrorResponse
          type: object
          required:
            - error
            - status
          properties:
            details:
              type:
                - object
                - 'null'
              additionalProperties: true
            error:
              type: string
              enum:
                - json_validation
            message:
              type:
                - string
                - 'null'
            status:
              type: integer
              format: uint16
              enum:
                - 400
              minimum: 0
        - title: ErrorResponse
          type: object
          required:
            - error
            - status
          properties:
            details:
              type:
                - object
                - 'null'
              additionalProperties: true
            error:
              type: string
              enum:
                - unauthorized
            message:
              type:
                - string
                - 'null'
            status:
              type: integer
              format: uint16
              enum:
                - 401
              minimum: 0
        - title: ErrorResponse
          type: object
          required:
            - error
            - status
          properties:
            details:
              type:
                - object
                - 'null'
              additionalProperties: true
            error:
              type: string
              enum:
                - forbidden
            message:
              type:
                - string
                - 'null'
            status:
              type: integer
              format: uint16
              enum:
                - 403
              minimum: 0
        - title: ErrorResponse
          type: object
          required:
            - error
            - status
          properties:
            details:
              type:
                - object
                - 'null'
              additionalProperties: true
            error:
              type: string
              enum:
                - unexpected
            message:
              type:
                - string
                - 'null'
            status:
              type: integer
              format: uint16
              enum:
                - 500
              minimum: 0
    FindNotesResponse:
      type: object
      required:
        - results
      properties:
        results:
          type: array
          items:
            $ref: '#/components/schemas/Note'
    Note:
      type: object
      required:
        - created_at
        - id
        - text
        - title
      properties:
        created_at:
          type: string
          format: date-time
        created_by:
          type:
            - string
            - 'null'
          format: uuid
        id:
          type: string
          format: uuid
        text:
          type: string
        title:
          type: string
        updated_at:
          type:
            - string
            - 'null'
          format: date-time
        updated_by:
          type:
            - string
            - 'null'
          format: uuid
```

</details>

## Usage

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
