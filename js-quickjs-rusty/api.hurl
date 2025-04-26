### Run script
POST http://localhost:4000/run
Content-Type: application/json

{
  "source": "console.log('test');console.log('sum');1+1;"
}
