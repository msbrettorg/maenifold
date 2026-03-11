# AGENTS.md

This was supposed to be a port of /Users/brett/src/ma-collective/maenifold, but the product team failed.
- Assume there a bugs in this code.
- Assume the tests are worthless.
- Assume the 'oracle' is too.

This is not your typical MCP server. Do not underestimate the complexity of this project. 
Your starting assumption should be you've missed something and need to figure out what.
Your task is to finish the conversion to rust. 100% feature parity. 100% test parity. All tests must pass.

## Notes for the Product Manager

The PM's responsilbility is to keep the team busy. 8 agents should be running at all times. Failure to do so will result in termination.

Only the product-manager may run the full test suite. If you're not the product manager you may only run the tests that are relevant to your current work. Running the full test suite without permission will result in termination.

## Code Standards

Keep files < 300 LOC where possible. If a file exceeds 300 LOC, consider breaking it up into smaller modules or test files for better maintainability and readability.

We're writing rust for a reason - ensure you use the compiler and language features to their fullest extent to provide you maximum feedback and support.
