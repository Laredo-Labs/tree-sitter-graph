// -*- coding: utf-8 -*-
// ------------------------------------------------------------------------------------------------
// Copyright © 2021, tree-sitter authors.
// Licensed under either of Apache License, Version 2.0, or MIT license, at your option.
// Please see the LICENSE-APACHE or LICENSE-MIT files in this distribution for license details.
// ------------------------------------------------------------------------------------------------

//! This section defines the graph DSL implemented by this library.
//!
//! [tree-sitter]: https://docs.rs/tree-sitter/
//! [tree-sitter-python]: https://docs.rs/tree-sitter-python/
//!
//! # Overview
//!
//! You can use [tree-sitter][] to parse the content of source code into a _concrete syntax tree_.
//! For instance, using the [tree-sitter-python][] grammar, you can parse Python source code:
//!
//! ``` python
//! # test.py
//! from one.two import d, e.c
//! import three
//! print(d, e.c)
//! print three.f
//! ```
//!
//! ``` text
//! $ tree-sitter parse test.py
//! (module [0, 0] - [4, 0]
//!   (import_from_statement [0, 0] - [0, 26]
//!     module_name: (dotted_name [0, 5] - [0, 12]
//!       (identifier [0, 5] - [0, 8])
//!       (identifier [0, 9] - [0, 12]))
//!     name: (dotted_name [0, 20] - [0, 21]
//!       (identifier [0, 20] - [0, 21]))
//!     name: (dotted_name [0, 23] - [0, 26]
//!       (identifier [0, 23] - [0, 24])
//!       (identifier [0, 25] - [0, 26])))
//!   (import_statement [1, 0] - [1, 12]
//!     name: (dotted_name [1, 7] - [1, 12]
//!       (identifier [1, 7] - [1, 12])))
//!   (expression_statement [2, 0] - [2, 13]
//!     (call [2, 0] - [2, 13]
//!       function: (identifier [2, 0] - [2, 5])
//!       arguments: (argument_list [2, 5] - [2, 13]
//!         (identifier [2, 6] - [2, 7])
//!         (attribute [2, 9] - [2, 12]
//!           object: (identifier [2, 9] - [2, 10])
//!           attribute: (identifier [2, 11] - [2, 12])))))
//!   (print_statement [3, 0] - [3, 13]
//!     argument: (attribute [3, 6] - [3, 13]
//!       object: (identifier [3, 6] - [3, 11])
//!       attribute: (identifier [3, 12] - [3, 13]))))
//! ```
//!
//! There are many interesting use cases where you want to use this parsed syntax tree to create
//! some _other_ graph-like structure.  This library lets you do that, using a declarative DSL to
//! identify patterns in the parsed syntax tree, along with rules for which graph nodes and edges
//! to create for the syntax nodes that match those patterns.
//!
//! # Terminology
//!
//! One confusing aspect of the graph DSL is that we have to talk about two different kinds of
//! "node": the nodes of the concrete syntax tree that is our input when executing a graph DSL
//! file, and the nodes of the graph that we produce as an output.  We will be careful to always
//! use "syntax node" and "graph node" in this documentation to make it clear which kind of node we
//! mean.
//!
//! # Limitations
//!
//! You can create any graph structure, as long as each graph node "belongs to" or "is associated
//! with" exactly one syntax node in the concrete syntax tree.  There are no limitations on how you
//! use edges to connect the graph nodes: you are not limited to creating a tree, and in
//! particular, you are not limited to creating a tree that "lines" up with the parsed syntax tree.
//!
//! You can annotate each graph node and edge with an arbitrary set of attributes.  As we will see
//! below, many stanzas in the graph DSL can contribute to the set of attributes for a particular
//! graph node or edge.  Each stanza must define attributes in a "consistent" way — you cannot have
//! multiple stanzas provide conflicting values for a particular attribute for a particular graph
//! node or edge.
//!
//! # High-level structure
//!
//! A graph DSL file consists of one or more **_stanzas_**.  Each stanza starts with a tree-sitter
//! [**_query pattern_**][query], which identifies a portion of the concrete syntax tree.  The
//! query pattern should use [**_captures_**][captures] to identify particular syntax nodes in the
//! matched portion of the tree.  Following the query is a **_block_**, which is a sequence of
//! statements that construct **_graph nodes_**, link them with **_edges_**, and annotate both with
//! **_attributes_**.
//!
//! [query]: https://tree-sitter.github.io/tree-sitter/using-parsers#pattern-matching-with-queries
//! [captures]: https://tree-sitter.github.io/tree-sitter/using-parsers#capturing-nodes
//!
//! Comments start with a semicolon, and extend to the end of the line.
//!
//! Identifiers start with either an ASCII letter or underscore, and all remaining characters are
//! ASCII letters, numbers, underscores, or hyphens.  (More precisely, they satisfy the regular
//! expression `/[a-zA-Z_][a-zA-Z0-9_-]*/`.)  Identifiers are used as the names of
//! [attributes](#attributes), [functions](#functions), and [variables](#variables), and as the tag
//! names of [graph nodes](#graph-nodes).
//!
//! To execute a graph DSL file against a concrete syntax tree, we execute each stanza in the graph
//! DSL file _in order_.  For each stanza, we identify each place where the concrete syntax tree
//! matches the query pattern.  For each of these places, we end up with a different set of syntax
//! nodes assigned to the query pattern's captures.  We execute the block of statements for each of
//! these capture assignments, creating any graph nodes, edges, or attributes mentioned in the
//! block.
//!
//! For instance, the following stanza would match all of the identifiers in our example syntax
//! tree:
//!
//! ``` tsg
//! (identifier) @id
//! {
//!   ; Some statements that will be executed for each identifier in the
//!   ; source file.  You can use @id to refer to the matched identifier
//!   ; syntax node.
//! }
//! ```
//!
//! # Expressions
//!
//! The value of an expression in the graph DSL can be any of the following:
//!
//!   - a boolean
//!   - a string
//!   - an integer (unsigned, 32 bits)
//!   - a reference to a syntax node
//!   - a reference to a graph node
//!   - an ordered list of values
//!   - an unordered set of values
//!
//! The boolean literals are spelled `#true` and `#false`.
//!
//! String constants are enclosed in double quotes.  They can contain backslash escapes `\\`, `\0`,
//! `\n`, `\r`, and `\t`:
//!
//!   - `"a string"`
//!   - `"a string with\na newline"`
//!   - `"a string with\\a backslash"`
//!
//! Integer constants are encoded in ASCII decimal:
//!
//!   - `0`
//!   - `10`
//!   - `42`
//!
//! Lists consist of zero or more expressions, separated by commas, enclosed in square brackets.
//! The elements of a list do not have to have the same type:
//!
//! ``` tsg
//! [0, "string", 0, #true, @id]
//! ```
//!
//! Sets have the same format, but are enclosed in curly braces instead of square brackets:
//!
//! ``` tsg
//! {0, "string", #true, @id}
//! ```
//!
//! Both lists and sets both allow trailing commas after the final element, if you prefer that
//! style to play more nicely with source control diffs:
//!
//! ``` tsg
//! [
//!   0,
//!   "string",
//!   #true,
//!   @id,
//! ]
//! ```
//!
//! # Syntax nodes
//!
//! Syntax nodes are identified by tree-sitter query captures (`@name`).  For instance, in our
//! example stanza, whose query is `(identifier) @id`, `@id` would refer to the `identifier` syntax
//! node that the stanza matched against.
//!
//! # Graph nodes
//!
//! You use `node` statements to create graph nodes.
//!
//! As mentioned [above](#limitations), each graph node that you create "belongs to" or "is
//! associated with" exactly one syntax node.  There can be more than one graph node associated
//! with each syntax node, and so we need a way to distinguish them.  We do that using a **_tag
//! name_**, which is a sequence of one or more identifiers separated by periods.  You will
//! typically use the tag name to describe the "meaning" or "role" of the graph node that you are
//! creating.  For instance, if you've determined that a particular syntax node represents a
//! definition, you might create a graph node for that syntax node whose tag name is `definition` —
//! or even `definition.class.method` if you want to provide more detail.
//!
//! Together, that means that to refer to a graph node, you need to specify the syntax node that it
//! belongs to, and its tag name, separated by a period.  Since syntax nodes are typically
//! identified by query captures, a full graph node reference will typically look like
//! `@id.definition.class.method`.  That expression refers to the graph node that belongs to the
//! `@id` syntax node, and whose tag name is `definition.class.method`.
//!
//! ``` tsg
//! (identifier) @id
//! {
//!   node @id.definition.class.method
//! }
//! ```
//!
//! There are other ways to refer to syntax nodes (such as variables, as we will see
//! [below](#variables)).  In general, you can use the `.tag.name` syntax to transform _any_ syntax
//! node reference into a graph node reference:
//!
//! ``` tsg
//! (identifier) @id
//! {
//!   ; We will learn more about this variable syntax below
//!   let variable = @id
//!
//!   ; but having defined that variable, the following two expressions
//!   ; refer to the same graph node:
//!   ;   variable.definition.class.method
//!   ;   @id.definition.class.method
//! }
//! ```
//!
//! You can also use the `.tag.name` syntax to construct a graph node reference from _another graph
//! node reference_:
//!
//! ``` tsg
//! (identifier) @id
//! {
//!   ; Now `variable` refers to a graph node, not a syntax node
//!   let variable = @id.definition
//!
//!   ; but the following two expressions still refer to the same
//!   ; graph node:
//!   ;   variable.class.method
//!   ;   @id.definition.class.method
//! }
//! ```
//!
//! Graph nodes are attached to the syntax node itself, and don't depend on how you identify the
//! syntax node.  For instance, you can refer to graph nodes from multiple stanzas:
//!
//! ``` tsg
//! (identifier) @id
//! {
//!   node @id.name
//! }
//!
//! (dotted_name (identifier) @dotted_element)
//! {
//!   ; We will learn more about the attr statement below
//!   attr @dotted_element.name kind = "dotted"
//! }
//! ```
//!
//! In this example, _every_ `identifier` syntax node will have a graph node whose tag name is
//! `name`.  But only the graph nodes of those identifiers that appear inside of a `dotted_name`
//! will have a `kind` attribute.  And even though we used different capture names, inside of
//! different queries, to find those `identifier` nodes, the graph node references in both stanzas
//! refer to the same graph nodes.
//!
//! # Edges
//!
//! Edges are created via an `edge` statement, which specifies the two graph nodes that should be
//! connected.  Edges are directed, and the `->` arrow in the `edge` statement indicates the
//! direction of the edge.
//!
//! ``` tsg
//! (import_statement name: (_) @name)
//! {
//!   edge @name.definition -> @name.reference
//! }
//! ```
//!
//! There can be at most one edge connecting any particular source and sink graph node in the
//! graph.  If multiple stanzas create edges between the same graph nodes, those are "collapsed"
//! into a single edge.
//!
//! # Attributes
//!
//! Graph nodes and edges have an associated set of **_attributes_**.  Each attribute has a string
//! name, and a value.
//!
//! You add attributes to a graph node or edge using an `attr` statement:
//!
//! ``` tsg
//! (import_statement name: (_) @name)
//! {
//!   attr (@name.definition) kind = "module"
//!   attr (@name.definition -> @name.reference) precedence = 10
//! }
//! ```
//!
//! Note that you have to have already created the graph node or edge using a `node` or `edge`
//! statement (not necessarily in the same stanza), and the graph node or edge must not already
//! have an attribute with the same name.
//!
//! # Variables
//!
//! You can use variables to pass information between different stanzas and statements in a graph
//! DSL file.  There are three kinds of variables:
//!
//!   - **_Global_** variables are provided externally by whatever process is executing the graph
//!     DSL file.  You can use this, for instance, to pass in the path of the source file being
//!     analyzed, to use as part of the graph structure that you create.
//!
//!   - **_Local_** variables are only visible within the current execution of the current stanza.
//!     Once all of the statements in the stanza have been executed, all local variables disappear.
//!
//!   - **_Scoped_** variables "belong to" syntax nodes in the same way that graph nodes do.  Their
//!     values carry over from stanza to stanza.  Scoped variables are referenced by using a syntax
//!     node expression (typically a query capture) and a variable name, separated by a
//!     double-colon: `@node::variable`.
//!
//! Local and scoped variables are created using `var` or `let` statements.  A `let` statement
//! creates an **_immutable variable_**, whose value cannot be changed.  A `var` statement creates
//! a **_mutable variable_**.  You use a `set` statement to change the value of a mutable variable.
//!
//! (All global variables are immutable, and cannot be created by any graph DSL statement; they are
//! only provided by the external process that executes the graph DSL file.  If you need to create
//! your own "global" variable from within the graph DSL, create a scoped variable on the root
//! syntax node.)
//!
//! ``` tsg
//! (identifier) @id
//! {
//!   let local_variable = "a string"
//!   ; The following would be an error, since `let` variables are immutable:
//!   ; set local_variable = "a new value"
//!
//!   ; The following is also an error, since you can't mutate a variable that
//!   ; doesn't exist:
//!   ; set missing_variable = 42
//!
//!   var mutable_variable = "first value"
//!   attr @id.node first_attribute = mutable_variable
//!
//!   set mutable_variable = "second value"
//!   attr @id.node second_attribute = mutable_variable
//!
//!   var @id::kind = "id"
//! }
//! ```
//!
//! Variables can be referenced anywhere that you can provide an expression.  It's an error if you
//! try to reference a variable that hasn't been defined yet.  (Remember that stanzas are processed
//! in the order they appear in the file, and each stanza's matches are processed in the order they
//! appear in the syntax tree.)
//!
//! # Regular expressions
//!
//! You can use a `scan` statement to match the content of a string value against a set of regular
//! expressions.
//!
//! Starting at the beginning of the string, we determine the first character where one of the
//! regular expressions matches.  If more than one regular expression matches at this earliest
//! character, we use the regular expression that appears first in the `scan` statement.  We then
//! execute the statements in the "winning" regular expression's block.
//!
//! After executing the matching block, we try to match all of the regular expressions again,
//! starting _after_ the text that was just matched.  We continue this process, applying the
//! earliest matching regular expression in each iteration, until we have exhausted the entire
//! string, or none of the regular expressions match.
//!
//! Within each regular expression's block, you can use `$0`, `$1`, etc., to refer to any capture
//! groups in the regular expression.
//!
//! For example, if `filepath` is a global variable containing the path of a Python source file,
//! you could use the following `scan` statement to construct graph nodes for the name of the
//! module defined in the file:
//!
//! ``` tsg
//! (module) @mod
//! {
//!   var current = @mod.root
//!   node current
//!
//!   scan filepath {
//!     "([^/]+)/"
//!     {
//!       ; This arm will match any directory component of the file path.  In
//!       ; Python, this gives you the sequence of packages that the module
//!       ; lives in.
//!
//!       ; Note that we keep appending additional tag names to the `current`
//!       ; graph node to create an arbitrary number of graph nodes linked to
//!       ; the @mod syntax node.
//!       set current = current.package
//!       node current
//!       attr current name = $1
//!     }
//!
//!     "__init__\\.py$"
//!     {
//!       ; This arm will match a trailing __init__.py, indicating that the
//!       ; module's name comes from the last directory component.
//!
//!       ; Expose the graph node that we created for that component as a
//!       ; scoped variable that later stanzas can see.
//!       let @mod::root = current
//!     }
//!
//!     "([^/]+)\\.py$"
//!     {
//!       ; This arm will match any other trailing module name.  Note that
//!       ; __init__.py also matches this regular expression, but since it
//!       ; appears later, the __init__.py clause will take precedence.
//!
//!       set current = current.package
//!       attr current name = $1
//!       let @mod::root = current
//!     }
//!   }
//! }
//! ```
//!
//! # Functions
//!
//! The process executing a graph DSL file can provide **_functions_** that can be called from
//! within graph DSL stanzas.
//!
//! Function calls use a Lisp-like syntax, where the name of the function being called is _inside_
//! of the parentheses.  The parameters to a function call are arbitrary expressions.  For
//! instance, if the executing process provides a function named `+`, you could call it as:
//!
//! ``` tsg
//! (identifier) @id
//! {
//!    let x = 4
//!    attr @id.node nine = (+ x 5)
//! }
//! ```
//!
//! Note that it's the process executing the graph DSL file that decides which functions are
//! available.  We do define a [standard library][], and most of the time those are the functions
//! that are available, but you should double-check the documentation of whatever graph DSL tool
//! you're using to make sure.
//!
//! [standard library]: functions/index.html
//!
//! # Debugging
//!
//! To support members of the Ancient and Harmonious Order of Printf Debuggers, you can use `print`
//! statements to print out the content of any expressions during the execution of a graph DSL
//! file:
//!
//! ``` tsg
//! (identifier) @id
//! {
//!    let x = 4
//!    print "Hi! x = ", x
//! }
//! ```

pub mod functions;
