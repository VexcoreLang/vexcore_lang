# Vexcore smoke test suite
# Expected output is documented below the file.

outln("== literals ==");
outln(123);
outln(3.5);
outln(true);
outln(null);

outln("== arithmetic ==");
outln(1 + 2 * 3);
outln(10 / 2);
outln(7 % 4);
outln(2 ** 5);
outln(-5 + 8);

outln("== variables ==");
let x = 10;
x += 5;
x -= 2;
x *= 3;
x /= 2;
outln(x);

outln("== functions ==");
fn inc(a: int): int {
    return a + 1;
}
fn add(a: int, b: int): int {
    return a + b;
}
outln(inc(41));
outln(add(2, 3));

outln("== conditionals ==");
if (true) {
    outln("if-true");
} els {
    outln("if-false");
}

if (false) {
    outln("if-no");
} els {
    outln("if-yes");
}

outln("== loops ==");
let w = 0;
while (w < 3) {
    w += 1;
}
outln(w);

let sum = 0;
for (i in 0..4) {
    sum += i;
}
outln(sum);

outln("== collections ==");
let xs = [1, 2, 3];
let obj = {"a": 1, "b": 2};
outln(xs[1]);
outln(obj["b"]);
outln(xs);
outln(obj);

outln("== strings ==");
let name = "Vexcore";
outln("Hello {name}");
outln("{name} works");

outln("== stdlib ==");
outln(math.abs(-3));
outln(math.min(2, 5));
outln(math.max(2, 5));
outln(math.clamp(15, 0, 10));
outln(math.pow(2, 5));
outln(math.sqrt(9));
outln(math.floor(3.9));
outln(math.ceil(3.1));
outln(math.round(3.6));
outln(math.trunc(3.9));
outln(math.sin(0));
outln(math.cos(0));
outln(math.tan(0));
outln(time.wait(0));

outln("== import ==");
use "test.vcl";
outln(test(41));

outln("== done ==");
