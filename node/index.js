// Wasm files must be loaded asynchronously, this import load the whole app asynchronously for simplicity
// import("./matcher").catch(err => console.log(err));
// require("./matcher").benchmark();

fastMatch = require("fast-match");

console.log("Wasm returned " + fastMatch.test());
