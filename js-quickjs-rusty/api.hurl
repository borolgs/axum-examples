### Run script
POST http://localhost:4000/run
Content-Type: application/json

{
  "code": "console.log('test');console.log('sum');1+1;"
}

### A + B
POST http://localhost:4000/sum
Content-Type: application/json

{"a": 2, "b": 2}

### Run compiled
POST http://localhost:4000/run
Content-Type: application/json

{
  "name": "sum",
  "args": { "a": 1, "b": 2 }
}
