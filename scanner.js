'use strict';

const {AhoCorasick} = require('./index.linux-x64-gnu.node');

const testPatterns = 'he him his she her hers '.split(' ').join('\x00');

class Scanner {
  constructor(array) {
    const arg = array.slice();
    arg.push('');
    this.patterns = array.slice();
    this.aho = new AhoCorasick(Buffer.from(arg.join('\x00')));
  }

  isSuspicious(buffer) {
    const indexes = this.aho.suspicious(buffer);
    if (!indexes) {
      return indexes;
    }

    indexes.sort();

    return indexes.map(ix => this.patterns[ix]);
  }
}

module.exports = Scanner;
