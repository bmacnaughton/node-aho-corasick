fs = require("fs");
AhoCorasick = require("ahocorasick");

let dataPath = "data/les-miserables.txt";
let wordsPath = "data/words.txt";

fs.readFile(dataPath, { encoding: "utf-8" }, (err, data) => {
  const lines = data.split("\n");
  fs.readFile(wordsPath, { encoding: "utf-8" }, (err, data) => {
    const words = data.split("\n");

    const regexes = [];
    for (const pattern in words) {
      const regex = new RegExp(pattern);
      regex.compile();
      regexes.push(regex);
    }

    for (i = 0; i < 1; i++) {
      for (line of lines) {
        match(regexes, line);
      }
    }
  });
});

function match(regexes, sentence) {
  const results = [];
  for (const regex of regexes) {
    if (regex.test(sentence)) {
      results.push(regex.toString());
    }
  }
  return results;
}
