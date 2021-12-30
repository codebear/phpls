# Strategy

## 1.pass Scan

Discover all symbols and declared types.

# OLD DOCS BELOW

# Analyzing

Analysis is performed on a per file basis. When analysing a file a reference to `SymbolStorage`
is available.

## Singlefile

Analysis can be performed on a single file, in this case only symbols from the current file will contribute.

## Repository/project

Analysis of a full repo/project is a complex task.

The use of a shared `SymbolStorage` makes this possible. Circular references between
code results in the need of running a multi-pass aproach.

# Type

## "Quality"

Types are handled according to how they are discovered

### Implicit

As an example, all scalar values have an implicit type. `42` is always an integer, `3.14` is always a float, "foobar" is always a string.

### Declared

A variable, method or parameter may have declared that is of a special type

```
private string $foo = "foo"
```

```
/**
* @var string
*/
```

There is no difference between a native php-declaration and a phpdoc-based one.
However, if both are present, referer to type-declaration-rules for details.

### Inferred

### Guessed

### Unknown

# How the analysis is performed

## Parsing

First a file is parsed using tree-sitter-php and a concrete syntax tree is constructed.

This is converted into an internal abstract-syntax-tree-representation in rust.

Then this three is scanned trough multiple passes. Each pass performs a specified part of the analysis

## Round one

The first pass is analyzing all basic "local" knowledge.

It can check that a single node is not in violation of any rules, naming conventions and so forth.
Rejecting usage of certain constructs.

This can not rely on any contextual knowledge, as this is probably not available.

This will register all declared classes, method, functions, constants and similar.

## Round two

This does more context- and state-aware analysis to more precisely determine
types of variables and return-types.

This pass might be ran multiple times until the symbol_data-stabilizes, or an a probability of an oscilation is detected

## Round three

The final pass is used to emit violations using the most precise type-information we got from the previous analysis

Round three can not modify the symbol-table
