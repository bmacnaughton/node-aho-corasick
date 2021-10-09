#!/usr/bin/env node
'use strict';

const {AhoCorasick} = require('./index.linux-x64-gnu.node');

const testPatterns = 'he him his she her hers '.split(' ').join('\x00');

const testData = 'bruce said hers was awesome';

class Scanner {
  constructor(array) {
    const arg = array.slice();
    arg.push('');
    this.patterns = array.slice();
    this.aho = new AhoCorasick(Buffer.from(arg.join('\x00')));
  }

  isSuspicious(bufferOrString) {
    if (typeof bufferOrString === 'string') {
      bufferOrString = Buffer.from(bufferOrString);
    } else if (!Buffer.isBuffer(bufferOrString)) {
      throw new Error('argument must be a buffer or string');
    }
    const indexes = this.aho.suspicious(bufferOrString);
    if (!indexes) {
      return indexes;
    }

    indexes.sort();

    return indexes.map(ix => this.patterns[ix]);
  }
}

module.exports = {
  Scanner,
  testPatterns,
  testData,
};

if (require.main) {
  const s = new Scanner('he him his she her hers'.split(' '));
  const r = s.isSuspicious(testData);
  if (!r) {
    console.log('pattern not found');
    process.exit(1);
  }
  if (r[0] != 'he' || r[1] !== 'her' || r[2] !== 'hers') {
    console.log('r', r, 'not equal to testData', testData);
    process.exit(1);
  }
  console.log('chicken test passed');
}
