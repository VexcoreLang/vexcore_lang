[setts]
cpu=1;
ram=1024;
mem=0;

[scenary]
let host = "8.8.8.8";
let alive = net.ping(host);
outln(f"host {host} alive = {alive}");

let numbers = [1, 2, 3];
for (i in 1..4) {
  outln(f"i = {i}");
}

let open = net.port_open("127.0.0.1", 22);
outln(f"ssh open = {open}");

let resolved = net.resolve("example.com");
outln(f"resolved = {resolved}");

outln(run("echo vexcore"));
