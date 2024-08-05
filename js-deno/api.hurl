### Run script
POST http://localhost:4000/run
Content-Type: application/json

{
  "source": "console.log('hello!'); function sum(a, b) { return a + b; } const a = { name: sum(1,1) }; export default a;"
}