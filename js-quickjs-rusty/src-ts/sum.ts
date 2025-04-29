declare var args: { a: number; b: number };

function sum(a: number, b: number): number {
  const res = a + b;
  console.log(`a + b = ${res}`);
  return res;
}

sum(args.a, args.b);
