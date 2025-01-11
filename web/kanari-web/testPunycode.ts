const punycode = require('punycode/');

// Example usage of punycode
const encoded = punycode.encode('mañana');
const decoded = punycode.decode(encoded);

console.log(`Encoded: ${encoded}`);
console.log(`Decoded: ${decoded}`);