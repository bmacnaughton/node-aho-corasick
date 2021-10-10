#!/usr/bin/env node
'use strict';

const {AhoCorasick} = require('./index.linux-x64-gnu.node');

const testPatterns = 'he him his she her hers '.split(' ').join('\x00');

const testData = 'bruce said hers was awesome';
const testUpper = 'BRUCE SAID HERS WAS AWESOME';

class Scanner {
  constructor(array) {
    // if passed an instance of ourselves then clone it.
    if (array instanceof Scanner) {
      this.patterns = array.patterns;
      this.aho = new AhoCorasick(array.aho);
      return;
    }
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

let errors = 0;
if (require.main) {
  const s1 = new Scanner('he him his she her hers'.split(' '));
  const r = s1.isSuspicious(testData);
  if (!r) {
    console.log('pattern not found');
    errors += 1;
  }
  if (r[0] != 'he' || r[1] !== 'her' || r[2] !== 'hers') {
    console.log('r', r, 'not equal to testData', testData);
    errors += 1;
  }
  const rU = s1.isSuspicious(testUpper);
  if (!rU) {
    console.log('uppercase did not match');
    errors += 1;
  }

  const s2 = new Scanner(s1);
  if (s1 === s2 || s1.aho === s2.aho) {
    console.log('instances are the same');
    errors += 1;
  }

  let threw = false;
  try {
    s3 = new Scanner();
  } catch (e) {
    threw = true;
  }
  if (!threw) {
    console.log('should have thrown');
    errors += 1;
  }

  if (errors) {
    console.log(`chicken test failed, ${errors} errors`);
    process.exit(1);
  }
  console.log('chicken test passed');
}
