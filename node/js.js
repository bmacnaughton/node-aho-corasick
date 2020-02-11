fs = require("fs");
AhoCorasick = require("ahocorasick");

let dataPath = "data/les-miserables.txt";
let wordsPath = "data/words.txt";

fs.readFile(dataPath, { encoding: "utf-8" }, (err, data) => {
  const lines = data.split("\n");
  fs.readFile(wordsPath, { encoding: "utf-8" }, (err, data) => {
    let words = data.split("\n");

    // Initialize the matcher
    let matcher = new AhoCorasick(words);

    for (i = 0; i < 50; i++) {
      for (line of lines) {
        matcher.search(line);
      }
    }
  });
});
