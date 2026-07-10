const { encode } = require('@toon-format/toon');
const fs = require('fs');

// Small dataset
const small = JSON.parse(fs.readFileSync('sample.json', 'utf-8'));
fs.writeFileSync('sample-output/sample.toon', encode(small) + '\n');

// Large dataset  
const large = JSON.parse(fs.readFileSync('sample_large.json', 'utf-8'));
fs.writeFileSync('sample-output/sample_large.toon', encode(large) + '\n');

// Spec example
const specExample = {
  name: "MyApp",
  version: "1.0.0",
  tags: ["rust", "performance"],
  users: [
    { id: 1, name: "Alice", email: "alice@ex.com" },
    { id: 2, name: "Bob", email: "bob@ex.com" }
  ]
};
fs.writeFileSync('sample-output/spec-example.toon', encode(specExample) + '\n');

console.log("=== Small ===");
console.log(encode(small));
console.log("\n=== Spec Example ===");
console.log(encode(specExample));
