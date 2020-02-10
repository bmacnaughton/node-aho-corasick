# Using node
NODE_ENV=production node --prof wasm.js 
node --prof-process isolate-0x49a1aa0-11836-v8.log > wasm.txt


# Using perf
perf record -g node --perf-basic-prof wasm.js
perf report --no-children