# {short title, representative of solved problem and found solution}

## Context and Problem Statement

HashMaps are slow, BTreeMaps are superior, but instead of implementing our own version I'm thinkingg of using the version in std::collections::BTreeMap.
The standard collections BTreeMap uses linear search, with plans to do smarter strategic search in the future, our implementation would aim to do that directly.

## Considered Options

* Use std::collections::BTreeMap
* Implement a BTreeMap ourselves with a smarter strategic search algo.

## Decision Outcome

Choosing option 1 for now as this isn't our current blocker.

### Consequences

* We will inevitably want to update this BTreeMap once it does become the blocker.