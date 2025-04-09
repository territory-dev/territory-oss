use testdir::testdir;

use territory_core::{pb_node_tokens, TokenKind};
use territory_core::territory::index::Location;
use clangrs::testlib::{ repr_diff, RepoWriter };


#[test]
fn class_file_entry() {
    let mut repo_writer = RepoWriter::new(&testdir!());
    repo_writer.add("defs.h",
r"class Foo {
    public:
        Foo();
        int bar(int x) const;

    private:
        void baz();

        int y;
};
").unwrap();
    repo_writer.add_cpp_unit("foo.cpp",
r#"#include "defs.h"


Foo::Foo() {
    this->y = 10;
}


int Foo::bar(int x) const {
    return 0;
}


void Foo::baz() {
}
"#).unwrap();
    repo_writer.write_clang_compile_commands().unwrap();

    let mut walker = repo_writer.index_repo();
    walker.follow_token("defs.h");

    let tokens = pb_node_tokens(walker.node());
    let got = tokens
        .iter()
        .map(|tok| tok.text.as_ref())
        .collect::<Vec<&str>>();
    let expected = vec![
        "class", " ", "Foo", " ", "{", " … ", "}", ";",
    ];
    assert_eq!(got, expected, "tokens don't match: {}", repr_diff(&expected, &got));
}

#[test]
fn class_node() {
    let mut repo_writer = RepoWriter::new(&testdir!());
    repo_writer.add("defs.h", r"
class Foo {
    public:
        Foo();
        int bar(int x) const;

    private:
        void baz();

        int y;
};
").unwrap();
    repo_writer.add_cpp_unit("foo.cpp", r#"
#include "defs.h"


Foo::Foo() {
    this->y = 10;
}


int Foo::bar(int x) const {
    return 0;
}


void Foo::baz() {
}
"#).unwrap();
    repo_writer.write_clang_compile_commands().unwrap();

    let mut walker = repo_writer.index_repo();
    walker.follow_token("defs.h");
    walker.follow_token("Foo");

    let structure_node = walker.node();
    // assert_eq!(structure_node.kind(), NodeKind::Structure);

    use TokenKind::*;
    let tokens = pb_node_tokens(structure_node);
    let got = tokens
            .iter()
            .map(|tok| (tok.type_, tok.text.as_ref()))
            .collect::<Vec<(TokenKind, &str)>>();
    let expected = vec![
        (Keyword, "class"), (WS, " "), (Identifier, "Foo"), (WS, " "), (Punctuation, "{"), (WS, "\n    "),
        (Keyword, "public"), (Punctuation, ":"), (WS, "\n        "),
        (Identifier, "Foo"), (Punctuation, "("), (Punctuation, ")"), (Punctuation, ";"), (WS, "\n        "),
        (Keyword, "int"), (WS, " "), (Identifier, "bar"), (Punctuation, "("), (Keyword, "int"), (WS, " "),
            (Identifier, "x"), (Punctuation, ")"), (WS, " "), (Keyword, "const"), (Punctuation, ";"),
            (WS, "\n\n    "),
        (Keyword, "private"), (Punctuation, ":"), (WS, "\n        "),
        (Keyword, "void"), (WS, " "), (Identifier, "baz"), (Punctuation, "("), (Punctuation, ")"),
            (Punctuation, ";"), (WS, "\n\n        "),
        (Keyword, "int"), (WS, " "), (Identifier, "y"), (Punctuation, ";"), (WS, "\n"),
        (Punctuation, "}")
    ];

    assert_eq!(got, expected, "tokens don't match: {}", repr_diff(&expected, &got));
}


#[test]
fn go_to_method_definition() {
    let mut repo_writer = RepoWriter::new(&testdir!());
    repo_writer.add("defs.h", r"
class Foo {
    public:
        int bar(int x) const;
};
").unwrap();
    repo_writer.add_cpp_unit("foo.cpp", r#"
#include "defs.h"


int Foo::bar(int x) const {
    return 0;
}
"#).unwrap();
    repo_writer.write_clang_compile_commands().unwrap();

    let mut walker = repo_writer.index_repo();
    walker.follow_token("defs.h");
    walker.follow_token("Foo");
    walker.follow_token("bar");
    assert!(walker.node().text.starts_with("int Foo::bar(int x) const {"));
}


#[test]
fn method_definition_file_entry() {
    let mut repo_writer = RepoWriter::new(&testdir!());
    repo_writer.add("defs.h",
r"class Foo {
    public:
        int bar(int x) const;
};
").unwrap();
    repo_writer.add_cpp_unit("foo.cpp",
r#"#include "defs.h"


int Foo::bar(int x) const {
    return 0;
}
"#).unwrap();
    repo_writer.write_clang_compile_commands().unwrap();

    let mut walker = repo_writer.index_repo();
    walker.follow_token("foo.cpp");

    let tokens = pb_node_tokens(walker.node());
    let got = tokens
        .iter()
        .map(|tok| (tok.type_, tok.text.as_ref()))
        .collect::<Vec<(TokenKind, &str)>>();
    use TokenKind::{Identifier, Punctuation, Keyword, WS, Literal};
    let expected = vec![
        ( Punctuation, "#",), ( Identifier, "include",), ( WS, " ",), ( Literal, "\"defs.h\"",), ( WS, "\n\n\n",),

        (Keyword, "int"), (WS, " "), (Identifier, "Foo"),
            (Punctuation, "::"), (Identifier, "bar"),
            (Punctuation, "("), (Keyword, "int"), (WS, " "), (Identifier, "x"), (Punctuation, ")"),
            (WS, " "), (Keyword, "const"), (WS, " "),
            (Punctuation, "{"), (WS, " … "), (Punctuation, "}"),
    ];

    assert_eq!(got, expected, "tokens don't match: {}", repr_diff(&expected, &got));
}


#[test]
fn method_definition_in_another_unit() {
    let mut repo_writer = RepoWriter::new(&testdir!());
    repo_writer.add("defs.h", r"
class Foo {
    public:
        int bar(int x) const;
};
").unwrap();
    repo_writer.add_cpp_unit("foo.cpp", r#"
#include "defs.h"


int Foo::bar(int x) const {
    return 0;
}
"#).unwrap();
    repo_writer.add_cpp_unit("main.cpp", r#"
#include "defs.h"


void main() {
    Foo c;
    return c.bar(1);
}
"#).unwrap();
    repo_writer.write_clang_compile_commands().unwrap();

    let mut walker = repo_writer.index_repo();
    walker.follow_token("main.cpp");
    walker.follow_token("main");
    walker.follow_token("bar");
    assert!(walker.node().text.starts_with("int Foo::bar(int x) const {"));
}


#[test]
fn inline_method_definition() {
    let mut repo_writer = RepoWriter::new(&testdir!());
    repo_writer.add_cpp_unit("main.cpp", r"
class Foo {
    private:
        // Comment
        int foo(int x) { return x; }
        int baz(int y) { return y; }
};
").unwrap();
    repo_writer.write_clang_compile_commands().unwrap();

    let mut walker = repo_writer.index_repo();
    walker.follow_token("main.cpp");
    let file_node_id = walker.node().id;
    assert_eq!(walker.node().member_of, None);

    walker.follow_token("Foo");
    assert_eq!(walker.node().member_of, None);
    assert_eq!(walker.node().container, Some(file_node_id));

    let tokens = pb_node_tokens(walker.node());
    let got = tokens
        .iter()
        .map(|tok| tok.text.as_ref())
        .collect::<Vec<&str>>();
    let expected = vec![
        "class", " ", "Foo", " ", "{",
        "\n    ", "private", ":",
        "\n        ", "// Comment",
        "\n        ", "int", " ", "foo", "(", "int", " ", "x", ")", " ", "{", " … ", "}",
        "\n        ", "int", " ", "baz", "(", "int", " ", "y", ")", " ", "{", " … ", "}",
        "\n", "}",
    ];
    assert_eq!(got, expected, "tokens don't match: {}", repr_diff(&expected, &got));

    let class_node_id = walker.node().id;

    walker.follow_token("baz");
    let method_node = walker.node();
    assert_eq!(method_node.text, "int baz(int y) { return y; }");
    assert_eq!(method_node.container, Some(class_node_id));
    assert_eq!(method_node.member_of, Some("Foo".to_string()));
}


#[test]
fn inline_method_use() {
    let mut repo_writer = RepoWriter::new(&testdir!());
    repo_writer.add_cpp_unit("main.cpp", r"
class C {
    int g(int x) { return x; }
};

void f() {
    C c;
    c.g(0);
}
").unwrap();
    repo_writer.write_clang_compile_commands().unwrap();

    let mut walker = repo_writer.index_repo();
    walker.follow_token("main.cpp");
    walker.follow_token("f");
    walker.follow_token("g");
    assert_eq!(walker.node().text, "int g(int x) { return x; }");

}


#[test]
fn namespace() {
    let mut repo_writer = RepoWriter::new(&testdir!());
    repo_writer.add("defs.h",
r"namespace foo {

class Foo {
    void bar();
};

}
").unwrap();
    repo_writer.add_cpp_unit("foo.cpp",
r#"#include "defs.h"

namespace foo {

void Foo::bar() { }

}
"#).unwrap();
    repo_writer.write_clang_compile_commands().unwrap();

    let mut walker = repo_writer.index_repo();
    walker.follow_token("defs.h");

    use TokenKind::*;
    let tokens = pb_node_tokens(walker.node());
    let got = tokens
            .iter()
            .map(|tok| (tok.type_, tok.text.as_ref()))
            .collect::<Vec<(TokenKind, &str)>>();
    let expected = vec![
        (Keyword, "namespace"), (WS, " "), (Identifier, "foo"), (WS, " "), (Punctuation, "{"), (WS, "\n\n"),
        (Keyword, "class"), (WS, " "), (Identifier, "Foo"), (WS, " "),
            (Punctuation, "{"), (WS, " … "), (Punctuation, "}"), (Punctuation, ";"), (WS, "\n\n"),
        (Punctuation, "}")
    ];

    assert_eq!(got, expected, "tokens don't match: {}", repr_diff(&expected, &got));
}


#[test]
fn nested_namespace() {
    let mut repo_writer = RepoWriter::new(&testdir!());
    repo_writer.add_cpp_unit("ns.cpp",
r"namespace foo {
    namespace baz {
        class Foo {
            void bar() {};
        };
    }
}
").unwrap();
    repo_writer.write_clang_compile_commands().unwrap();

    let mut walker = repo_writer.index_repo();
    walker.follow_token("ns.cpp");

    assert_eq!(walker.node().text,
r"namespace foo {
    namespace baz {
        class Foo { … };
    }
}");


    walker.follow_token("Foo");
    assert_eq!(walker.node().text,
r"class Foo {
            void bar() { … };
        }");
}


#[test]
fn namespace_reference() {
    let mut repo_writer = RepoWriter::new(&testdir!());
    repo_writer.add_cpp_unit("ns.cpp", r"
namespace foo {
    class Foo {};
}

foo::Foo x;
").unwrap();
    repo_writer.write_clang_compile_commands().unwrap();

    let mut walker = repo_writer.index_repo_with_args(|args| { args.fatal_missing_spans = true; });
    walker.follow_token("ns.cpp");
    walker.follow_token("x");

    use TokenKind::*;
    let tokens = pb_node_tokens(walker.node());
    let got = tokens
            .iter()
            .map(|tok| (tok.type_, tok.text.as_ref()))
            .collect::<Vec<(TokenKind, &str)>>();
    let expected = vec![
        (Identifier, "foo"), (Punctuation, "::"), (Identifier, "Foo"), (WS, " "), (Identifier, "x")
    ];

    assert_eq!(got, expected, "tokens don't match: {}", repr_diff(&expected, &got));
}


#[test]
fn sepples_angle_include() {
    let mut repo_writer = RepoWriter::new(&testdir!());
    repo_writer.add_cpp_unit("mod.c", r#"
#include <stddef.h>

void f() {
    size_t s;
}
"#).unwrap();
    repo_writer.write_clang_compile_commands().unwrap();

    let mut walker = repo_writer.index_repo();
    walker.follow_token("mod.c");
    walker.follow_token("f");
    // walker.follow_token("size_t");
    // dbg!(&walker.node().path, &walker.node().text);
    // assert!(false);
}



#[test]
fn sepples_angle_include_secondary() {
    let mut repo_writer = RepoWriter::new(&testdir!());
    repo_writer.add("defs.h", r#"
#include <cstddef>
"#).unwrap();
    repo_writer.add_cpp_unit("mod.c", r#"
#include "defs.h"

void f() {
    size_t s;
}
"#).unwrap();
    repo_writer.write_clang_compile_commands().unwrap();

    let mut walker = repo_writer.index_repo();
    walker.follow_token("mod.c");
    walker.follow_token("f");
}


#[test]
fn extern_() {
    let mut repo_writer = RepoWriter::new(&testdir!());
    repo_writer.add_cpp_unit("main.cpp",
r#"extern "C" {

void f() { }

}
"#).unwrap();
    repo_writer.write_clang_compile_commands().unwrap();

    let mut walker = repo_writer.index_repo();
    walker.follow_token("main.cpp");

    use TokenKind::*;
    let tokens = pb_node_tokens(walker.node());
    let got = tokens
            .iter()
            .map(|tok| (tok.type_, tok.text.as_ref()))
            .collect::<Vec<(TokenKind, &str)>>();
    let expected = vec![
        ( Keyword, "extern",), ( WS, " ",), ( Literal, "\"C\"",),
            ( WS, " ",), ( Punctuation, "{",),
        ( WS, "\n\n",),

        (Keyword, "void"), (WS, " "), (Identifier, "f"), (Punctuation, "("),(Punctuation, ")"),  (WS, " "),
            (Punctuation, "{"), (WS, " … "), (Punctuation, "}"),

        (WS, "\n\n"), (Punctuation, "}"),
    ];
    assert_eq!(got, expected, "tokens don't match: {}", repr_diff(&expected, &got));

    walker.follow_token("f");
    assert!(walker.node().text.starts_with("void f() { }"));
}



#[test]
fn extern_macro() {
    let mut repo_writer = RepoWriter::new(&testdir!());
    repo_writer.add_cpp_unit("main.cpp",
r#"#define XT extern "C" {

XT

void f() { }

}
"#).unwrap();
    repo_writer.write_clang_compile_commands().unwrap();

    let mut walker = repo_writer.index_repo();
    walker.follow_token("main.cpp");

    use TokenKind::*;
    let tokens = pb_node_tokens(walker.node());
    let got = tokens
            .iter()
            .map(|tok| (tok.type_, tok.text.as_ref()))
            .collect::<Vec<(TokenKind, &str)>>();
    let expected = vec![
        (Keyword, "#"), (Keyword, "define"), (WS, " "), (Identifier, "XT"), (WS, " …"), (WS, "\n\n"),

        (Identifier, "XT"), (WS, "\n\n"),

        (Keyword, "void"), (WS, " "), (Identifier, "f"), (Punctuation, "("),(Punctuation, ")"),  (WS, " "),
            (Punctuation, "{"), (WS, " … "), (Punctuation, "}"),

        (WS, "\n\n"), (Punctuation, "}"),
    ];
    assert_eq!(got, expected, "tokens don't match: {}", repr_diff(&expected, &got));

    walker.follow_token("f");
    assert!(walker.node().text.starts_with("void f() { }"));
}



#[test]
fn method_definition_across_modules() {
    let mut repo_writer = RepoWriter::new(&testdir!());
    repo_writer.add("defs.h", r"
class A { void f(); };
class B { void g(); };
").unwrap();
    repo_writer.add_cpp_unit("a.cpp", r#"
#include "defs.h"
void A::f() { }
"#).unwrap();
    repo_writer.add_cpp_unit("b.cpp", r#"
#include "defs.h"
void B::g() { }
"#).unwrap();
    repo_writer.write_clang_compile_commands().unwrap();

    let mut walker = repo_writer.index_repo();
    walker.follow_token("defs.h");
    walker.follow_token("A");
    walker.follow_token("f");
    assert!(walker.node().text.starts_with("void A::f() {"));
    walker.reset();
    walker.follow_token("defs.h");
    walker.follow_token("B");
    walker.follow_token("g");
    assert!(walker.node().text.starts_with("void B::g() {"));
}


#[test]
fn data_member() {
    let mut repo_writer = RepoWriter::new(&testdir!());
    repo_writer.add("defs.h", r"
class A {
    int x;
    void f();
};
").unwrap();
    repo_writer.add_cpp_unit("a.cpp", r#"
#include "defs.h"
void A::f() {
    x = 1;
}
"#).unwrap();
    repo_writer.write_clang_compile_commands().unwrap();

    let mut walker = repo_writer.index_repo();
    walker.follow_token("defs.h");
    walker.follow_token("A");
    let class_node_id = walker.node().id;
    walker.reset();
    walker.follow_token("a.cpp");
    walker.follow_token("f");
    walker.follow_token("x");
    assert_eq!(walker.node().id, class_node_id);
}


#[test]
fn data_member_references() {
    let mut repo_writer = RepoWriter::new(&testdir!());
    repo_writer.add("defs.h", r"
class A {
    int x;
    void f();
};
").unwrap();
    repo_writer.add_cpp_unit("a.cpp", r#"
#include "defs.h"
void A::f() {
    x = 1;
}
"#).unwrap();
    repo_writer.write_clang_compile_commands().unwrap();

    let mut walker = repo_writer.index_repo();
    walker.follow_token("defs.h");
    walker.follow_token("A");
    let field_tok_refs = walker.token_references("x");

    assert_eq!(field_tok_refs.refs.len(), 1);
    let ref_ = &field_tok_refs.refs[0];
    assert_eq!(ref_.use_path, "a.cpp");
    assert_eq!(ref_.use_location, Some(Location {
        line: 4,
        column: 5,
        offset: 37,
    }));
    assert_eq!(ref_.context, "A::f");

    walker.go_to_node(field_tok_refs.refs[0].href);

    assert!(walker.node().text.starts_with("void A::f() {"));
}


#[test]
fn inline_constructor() {
    let mut repo_writer = RepoWriter::new(&testdir!());
    repo_writer.add_cpp_unit("a.cpp", r#"
class A {
    int x;
public:
    A(int y) : x(y) { }
};
"#).unwrap();
    repo_writer.write_clang_compile_commands().unwrap();

    let mut walker = repo_writer.index_repo();
    walker.follow_token("a.cpp");
    walker.follow_token("A");
    assert_eq!(walker.node().text, r#"class A {
    int x;
public:
    A(int y) : … { … }
}"#);
    walker.follow_nth_token("A", 2);
    assert_eq!(walker.node().text, r#"A(int y) : x(y) { }"#);
}


#[test]
fn constructor() {
    let mut repo_writer = RepoWriter::new(&testdir!());
    repo_writer.add_cpp_unit("a.cpp", r#"
class A {
    int x;
public:
    A(int y);
};

A::A(int y) : x(y) { return 0; }
"#).unwrap();
    repo_writer.write_clang_compile_commands().unwrap();

    let mut walker = repo_writer.index_repo();
    walker.follow_token("a.cpp");
    walker.follow_token("A");
    walker.follow_nth_token("A", 2);
    assert_eq!(walker.node().text, r#"A::A(int y) : x(y) { return 0; }"#);
}


#[test]
fn extern_constructor() {
    let mut repo_writer = RepoWriter::new(&testdir!());
    repo_writer.add_cpp_unit("main.cpp", r#"
#include "a.h"
int main() {
    A a(1);
}
"#).unwrap();

    repo_writer.add_cpp_unit("a.cpp", r#"
#include "a.h"

A::A(int x) { (void)x; }
"#).unwrap();
    repo_writer.add("a.h", r#"
class A {
public:
    explicit A(int x);
};
"#).unwrap();
    repo_writer.write_clang_compile_commands().unwrap();

    let mut walker = repo_writer.index_repo();
    walker.follow_token("a.h");
    walker.follow_token("A");
    walker.follow_nth_token("A", 2);
    assert_eq!(walker.node().text, r"A::A(int x) { (void)x; }");
}


#[test]
fn extern_constructor_in_nested_class() {
    let mut repo_writer = RepoWriter::new(&testdir!());
    repo_writer.add_cpp_unit("main.cpp", r#"
#include "a.h"
int main() {
    A a(1);
}
"#).unwrap();

    repo_writer.add_cpp_unit("a.cpp", r#"
#include "a.h"

B::A::A(int x) { (void)x; }
"#).unwrap();
    repo_writer.add("a.h", r#"
class B {
private:
    class A {
    public:
        explicit A(int x);
    };
};
"#).unwrap();
    repo_writer.write_clang_compile_commands().unwrap();

    let mut walker = repo_writer.index_repo();
    walker.follow_token("a.h");
    walker.follow_token("B");
    walker.follow_nth_token("A", 2);
    assert_eq!(walker.node().text, r"B::A::A(int x) { (void)x; }");
}


#[test]
fn inline_destructor() {
    let mut repo_writer = RepoWriter::new(&testdir!());
    repo_writer.add_cpp_unit("a.cpp", r#"
class A {
    int x;
public:
    ~A() { x = 0; }
};
"#).unwrap();
    repo_writer.write_clang_compile_commands().unwrap();

    let mut walker = repo_writer.index_repo();
    walker.follow_token("a.cpp");
    walker.follow_token("A");
    assert_eq!(walker.node().text, r#"class A {
    int x;
public:
    ~A() { … }
}"#);
    walker.follow_nth_token("A", 2);
    assert_eq!(walker.node().text, r#"~A() { x = 0; }"#);
}


#[test]
fn externally_defined_destructor() {
    let mut repo_writer = RepoWriter::new(&testdir!());
    repo_writer.add_cpp_unit("a.cpp", r#"
class A {
    int x;
public:
    ~A();
};

A::~A() { x = 0; }
"#).unwrap();
    repo_writer.write_clang_compile_commands().unwrap();

    let mut walker = repo_writer.index_repo();
    walker.follow_token("a.cpp");
    walker.follow_token("A");
    walker.follow_nth_token("A", 2);
    assert_eq!(walker.node().text, r#"A::~A() { x = 0; }"#);
}


#[test]
fn instance_member() {
    let mut repo_writer = RepoWriter::new(&testdir!());
    repo_writer.add("defs.h", r"
class B {
    void g();
};
class A {
    B b;
    void f();
};
").unwrap();
    repo_writer.add_cpp_unit("a.cpp", r#"
#include "defs.h"
void A::f() {
    b.g();
}
"#).unwrap();
    repo_writer.add_cpp_unit("b.cpp", r#"
#include "defs.h"
void B::g() { }
"#).unwrap();

    repo_writer.write_clang_compile_commands().unwrap();

    let mut walker = repo_writer.index_repo();
    walker.follow_token("defs.h");
    walker.follow_token("A");
    let class_node_id = walker.node().id;
    walker.follow_token("f");
    walker.follow_token("b");
    assert_eq!(walker.node().id, class_node_id);

    walker.back().unwrap();
    walker.follow_token("g");
    assert!(walker.node().text.starts_with("void B::g() {"));
}


#[test]
fn class_template_file_entry() {
    let mut repo_writer = RepoWriter::new(&testdir!());
    repo_writer.add_cpp_unit("a.cpp",
r#"template <class T>
class C {
    T* g(T *x) { return x; }
};
"#).unwrap();

    repo_writer.write_clang_compile_commands().unwrap();

    let mut walker = repo_writer.index_repo_with_args(|args| { args.fatal_missing_spans = true; });
    walker.follow_token("a.cpp");
    assert_eq!(walker.node().text,
r"template <class T>
class C { … };");
}


#[test]
fn class_template_inline_method_class_entry() {
    let mut repo_writer = RepoWriter::new(&testdir!());
    repo_writer.add_cpp_unit("a.cpp", r#"
template <class T>
class C {
    T* g(T *x) { return x; }
};
"#).unwrap();

    repo_writer.write_clang_compile_commands().unwrap();

    let mut walker = repo_writer.index_repo_with_args(|args| { args.fatal_missing_spans = true; });
    walker.follow_token("a.cpp");
    walker.follow_token("C");

    let tokens = pb_node_tokens(walker.node());
    let got = tokens
        .iter()
        .map(|tok| (tok.type_, tok.text.as_ref()))
        .collect::<Vec<(TokenKind, &str)>>();
    use TokenKind::{Identifier, Punctuation, Keyword, WS};
    let expected = vec![
        (Keyword, "template"), (WS, " "),
            (Punctuation, "<"),
                (Keyword, "class"), (WS, " "), (Identifier, "T"),
            (Punctuation, ">"), (WS, "\n"),

        (Keyword, "class"), (WS, " "), (Identifier, "C"), (WS, " "), (Punctuation, "{"),

        (WS, "\n    "), (Identifier, "T"), (Punctuation, "*"), (WS, " "),
            (Identifier, "g"),
            (Punctuation, "("),
                (Identifier, "T"),  (WS, " "), (Punctuation, "*"), (Identifier, "x"),
            (Punctuation, ")"), (WS, " "),
            (Punctuation, "{"), (WS, " … "), (Punctuation, "}"),

        (WS, "\n"), (Punctuation, "}"),
    ];
    assert_eq!(got, expected, "tokens don't match: {}", repr_diff(&expected, &got));
}


#[test]
fn class_template_inline_method() {
    let mut repo_writer = RepoWriter::new(&testdir!());
    repo_writer.add_cpp_unit("a.cpp", r#"
template <class T>
class C {
    T* g(T *x) { return x; }
};
"#).unwrap();

    repo_writer.write_clang_compile_commands().unwrap();

    let mut walker = repo_writer.index_repo_with_args(|args| { args.fatal_missing_spans = true; });
    walker.follow_token("a.cpp");
    walker.follow_token("C");
    walker.follow_token("g");
    assert_eq!(walker.node().text, "T* g(T *x) { return x; }");
}


#[test]
fn class_template_inline_template_method() {
    let mut repo_writer = RepoWriter::new(&testdir!());
    repo_writer.add_cpp_unit("a.cpp", r#"
template <class T>
class C {
    template<class U> void f(T &t, U &u) { return; }
};
"#).unwrap();

    repo_writer.write_clang_compile_commands().unwrap();

    let mut walker = repo_writer.index_repo_with_args(|args| { args.fatal_missing_spans = true; });
    walker.follow_token("a.cpp");
    walker.follow_token("C");
    walker.follow_token("f");
    assert_eq!(walker.node().text, "template<class U> void f(T &t, U &u) { return; }");
}


#[test]
fn class_template_inline_template_method_class_entry() {
    let mut repo_writer = RepoWriter::new(&testdir!());
    repo_writer.add_cpp_unit("a.cpp", r#"
template <class T>
class C {
    template<class U> void f(T &t, U &u) { return; }
};
"#).unwrap();

    repo_writer.write_clang_compile_commands().unwrap();

    let mut walker = repo_writer.index_repo_with_args(|args| { args.fatal_missing_spans = true; });
    walker.follow_token("a.cpp");
    walker.follow_token("C");

    let tokens = pb_node_tokens(walker.node());
    let got = tokens
        .iter()
        .map(|tok| (tok.type_, tok.text.as_ref()))
        .collect::<Vec<(TokenKind, &str)>>();
    use TokenKind::{Identifier, Punctuation, Keyword, WS};
    let expected = vec![
        (Keyword, "template"), (WS, " "),
            (Punctuation, "<"),
                (Keyword, "class"), (WS, " "), (Identifier, "T"),
            (Punctuation, ">"), (WS, "\n"),

        (Keyword, "class"), (WS, " "), (Identifier, "C"), (WS, " "), (Punctuation, "{"),

        (WS, "\n    "),
            (Keyword, "template"), (Punctuation, "<"),
                (Keyword, "class"), (WS, " "), (Identifier, "U"),
            (Punctuation, ">"), (WS, " "),
            (Keyword, "void"), (WS, " "), (Identifier, "f"),
            (Punctuation, "("),
                (Identifier, "T"),  (WS, " "), (Punctuation, "&"), (Identifier, "t"),
                (Punctuation, ","), (WS, " "),
                (Identifier, "U"),  (WS, " "), (Punctuation, "&"), (Identifier, "u"),
            (Punctuation, ")"), (WS, " "),
            (Punctuation, "{"), (WS, " … "), (Punctuation, "}"),

        (WS, "\n"), (Punctuation, "}"),
    ];
    assert_eq!(got, expected, "tokens don't match: {}", repr_diff(&expected, &got));
}


#[test]
fn templated_instance_member_reference() {
    let mut repo_writer = RepoWriter::new(&testdir!());
    repo_writer.add("defs.h", r"
class C {};

template <class T>
class B {
public:
    T* g(T *x) { return x; }
};

class A {
    B<C> b;
    void f();
};
").unwrap();
    repo_writer.add_cpp_unit("a.cpp", r#"
#include "defs.h"
void A::f() {
    C c;
    b.g(&c);
}
"#).unwrap();

    repo_writer.write_clang_compile_commands().unwrap();

    let mut walker = repo_writer.index_repo_with_args(|args| { args.fatal_missing_spans = true; });
    walker.follow_token("defs.h");
    walker.follow_token("A");
    walker.follow_token("f");
    walker.follow_token("g");
    assert_eq!(walker.node().text, "T* g(T *x) { return x; }");
}


#[test]
fn struct_template() {
    let mut repo_writer = RepoWriter::new(&testdir!());
    repo_writer.add_cpp_unit("a.cpp", r#"
class C {};

template <class T>
struct S {
    T* g(T *x) { return x; }
};

void f() {
    S<C> s;
    s.g();
}
"#).unwrap();

    repo_writer.write_clang_compile_commands().unwrap();

    let mut walker = repo_writer.index_repo_with_args(|args| { args.fatal_missing_spans = true; });
    walker.follow_token("a.cpp");
    walker.follow_token("f");
    walker.follow_token("g");
    assert_eq!(walker.node().text, "T* g(T *x) { return x; }");
}


#[test]
fn namespaced_enum() {
    let mut repo_writer = RepoWriter::new(&testdir!());
    repo_writer.add("defs.h", r"
namespace N {};
").unwrap();
    repo_writer.add_cpp_unit("a.cpp", r#"
#include "defs.h"
using namespace N;
enum E { E1, E2 };
"#).unwrap();

    repo_writer.write_clang_compile_commands().unwrap();

    let mut walker = repo_writer.index_repo();
    walker.follow_token("a.cpp");
    walker.follow_token("E");

    let tokens = pb_node_tokens(walker.node());
    let got = tokens
        .iter()
        .map(|tok| (tok.type_, tok.text.as_ref()))
        .collect::<Vec<(TokenKind, &str)>>();
    use TokenKind::{Identifier, Punctuation, Keyword, WS};
    let expected = vec![
        (Keyword, "enum"),
        (WS, " "),
        (Identifier, "E"),
        (WS, " "),
        (Punctuation, "{"),
        (WS, " "),
        (Identifier, "E1"),
        (Punctuation, ","),
        (WS, " "),
        (Identifier, "E2"),
        (WS, " "),
        (Punctuation, "}"),
    ];
    assert_eq!(got, expected, "tokens don't match: {}", repr_diff(&expected, &got));
}


#[test]
fn namespaced_enum_file_entry() {
    let mut repo_writer = RepoWriter::new(&testdir!());
    repo_writer.add("defs.h",
r"namespace N {};
").unwrap();
    repo_writer.add_cpp_unit("a.cpp",
r#"#include "defs.h"
using namespace N;
enum E { E1, E2 };
"#).unwrap();

    repo_writer.write_clang_compile_commands().unwrap();

    let mut walker = repo_writer.index_repo();
    walker.follow_token("a.cpp");

    let tokens = pb_node_tokens(walker.node());
    let got = tokens
        .iter()
        .map(|tok| (tok.type_, tok.text.as_ref()))
        .collect::<Vec<(TokenKind, &str)>>();
    use TokenKind::{Identifier, Punctuation, Keyword, WS, Literal};
    let expected = vec![
        ( Punctuation, "#",), ( Identifier, "include",), ( WS, " ",), ( Literal, "\"defs.h\"",), ( WS, "\n",),

        (Keyword, "using"), (Keyword, "namespace"), (Identifier, "N"), ( Punctuation, ";",), ( WS, "\n",),

        (Keyword, "enum"), (WS, " "), (Identifier, "E"), (WS, " "),
            (Punctuation, "{"), (WS, " … "), (Punctuation, "}"), (Punctuation, ";")
    ];
    assert_eq!(got, expected, "tokens don't match: {}", repr_diff(&expected, &got));
}


#[test]
fn namespaced_class_templated_instance_member_reference() {
    let mut repo_writer = RepoWriter::new(&testdir!());
    repo_writer.add("defs.h", r"
namespace N {

class C {};

template <class T>
class B {
public:
    T* g(T *x) { return x; }
};

class A {
    B<C> b;
    void f();
};

}
").unwrap();
    repo_writer.add_cpp_unit("a.cpp", r#"
#include "defs.h"

using namespace N;

void A::f() {
    C c;
    b.g(&c);
}
"#).unwrap();

    repo_writer.write_clang_compile_commands().unwrap();

    let mut walker = repo_writer.index_repo();
    walker.follow_token("defs.h");
    walker.follow_token("A");
    walker.follow_token("f");
    walker.follow_token("g");
    assert_eq!(walker.node().text, "T* g(T *x) { return x; }");
}


#[test]
fn assignment_in_if() {
    let mut repo_writer = RepoWriter::new(&testdir!());
    repo_writer.add_cpp_unit("mod.cpp", r#"
class C {
    C *g() { return 0; }
};

int f() {
    C x;
    if (C *y = x.g())
        return 1;
    return 0;
}
"#).unwrap();

    repo_writer.write_clang_compile_commands().unwrap();

    let mut walker = repo_writer.index_repo();
    walker.follow_token("mod.cpp");
    walker.follow_token("f");
    let mut after_x = false;
    walker.follow_token_by(&mut |tok| {
        if tok.text == "x" { after_x = true; }
        if !after_x { return false; }
        tok.text == "g"
    }).unwrap();

    assert_eq!(walker.node().text, "C *g() { return 0; }");
}


#[test]
fn macro_return_type_of_nested_func() {
    let mut repo_writer = RepoWriter::new(&testdir!());
    repo_writer.add_cpp_unit("mod.cpp", r#"
#define T int

class C {
    T *g() { return 0; }
};
"#).unwrap();

    repo_writer.write_clang_compile_commands().unwrap();

    let mut walker = repo_writer.index_repo();
    walker.follow_token("mod.cpp");
    walker.follow_token("C");
    walker.follow_token("g");

    assert_eq!(walker.node().text, "T *g() { return 0; }");
}


#[test]
fn nested_defintions_with_use_file_entry() {
    let mut repo_writer = RepoWriter::new(&testdir!());
    repo_writer.add_cpp_unit("mod.cpp",
r#"class A {
    struct B {
        void f() { A a; }
    } b;
};
"#).unwrap();

    repo_writer.write_clang_compile_commands().unwrap();

    let mut walker = repo_writer.index_repo();
    walker.follow_token("mod.cpp");

    let tokens = pb_node_tokens(walker.node());
    let got = tokens
        .iter()
        .map(|tok| tok.text.as_ref())
        .collect::<Vec<&str>>();
    let expected = vec![
        "class", " ", "A", " ", "{", " … ", "}", ";"
    ];
    assert_eq!(got, expected, "tokens don't match: {}", repr_diff(&expected, &got));
}


#[test]
#[ignore]
fn include_from_clang_resources() {
    let mut repo_writer = RepoWriter::new(&testdir!());
    repo_writer.add_cpp_unit("mod.cpp", r#"
#include <cstddef>
using namespace std;
size_t x;
"#).unwrap();

    repo_writer.write_clang_compile_commands().unwrap();

    let mut walker = repo_writer.index_repo();
    walker.follow_token("mod.cpp");
    walker.follow_token("size_t");
    walker.follow_token("size_t");
    assert!(walker.node().text.starts_with("size_t x"));
}


#[test]
#[ignore]
fn method_in_nested_template() {
    let mut repo_writer = RepoWriter::new(&testdir!());
    repo_writer.add_cpp_unit("mod.cpp", r#"
template <class _A, class _B, class _C>
class Cls {
public:
  typedef _A a;
  typedef _B b;
  typedef _C c;

  template <class _D>
  void fun(_D d1, _D d2);
};

template <class _A, class _B, class _C>
template <class _D>
void Cls<_A, _B, _C>::fun(_D d1, _D d2) {
    a aa;
    _D d;
}
"#).unwrap();

    repo_writer.write_clang_compile_commands().unwrap();

    let mut walker = repo_writer.index_repo_with_args(|args| { args.fatal_missing_spans = true; });
    walker.follow_token("mod.cpp");
    walker.follow_token("fun(_D, _D)");  // FIXME
    assert!(walker.node().text.starts_with("template <class _A, class _B, class _C>
template <class _D>
void Cls<_A, _B, _C>::fun(_D d1, _D d2) {"), "got {}", &walker.node().text);


}



#[test]
#[ignore]
fn method_in_nested_template_namespaced() {
    let mut repo_writer = RepoWriter::new(&testdir!());
    repo_writer.add_cpp_unit("mod.cpp", r#"
namespace NS {

template <class _A, class _B, class _C>
class Cls {
public:
  typedef _A a;
  typedef _B b;
  typedef _C c;

  template <class _D>
  void fun(_D d1, _D d2);
};

template <class _A, class _B, class _C>
template <class _D>
void Cls<_A, _B, _C>::fun(_D d1, _D d2) {
    a aa;
    _D d;
}

}
"#).unwrap();

    repo_writer.write_clang_compile_commands().unwrap();

    let mut walker = repo_writer.index_repo_with_args(|args| { args.fatal_missing_spans = true; });
    walker.follow_token("mod.cpp");
    walker.follow_token("fun(_D, _D)");  // FIXME
    assert!(walker.node().text.starts_with("template <class _A, class _B, class _C>
template <class _D>
void Cls<_A, _B, _C>::fun(_D d1, _D d2) {"), "got {}", &walker.node().text);


}


#[test]
fn multiline_nested_method_head() {
    let mut repo_writer = RepoWriter::new(&testdir!());
    repo_writer.add_cpp_unit("mod.cpp", r#"
class C {
    int f(
        int x,
        int y
    ) {
        return x + y;
    }
};
"#).unwrap();

    repo_writer.write_clang_compile_commands().unwrap();

    let mut walker = repo_writer.index_repo();
    walker.follow_token("mod.cpp");
    walker.follow_token("C");

    let tokens = pb_node_tokens(walker.node());
    let got = tokens
        .iter()
        .map(|tok| tok.text.as_ref())
        .collect::<Vec<&str>>();
    let expected = vec![
        "class", " ", "C", " ", "{",
        "\n    ", "int", " ", "f", "(",
        "\n        ", "int", " ", "x", ",",
        "\n        ", "int", " ", "y",
        "\n    ", ")", " ", "{", " … ", "}", "\n",
        "}",
    ];
    assert_eq!(got, expected, "tokens don't match: {}", repr_diff(&expected, &got));
}



#[test]
fn friend() {
    let mut repo_writer = RepoWriter::new(&testdir!());
    repo_writer.add_cpp_unit("mod.cpp", r#"
class F;

class C {
    friend class F;
};

class F { C c; };
"#).unwrap();

    repo_writer.write_clang_compile_commands().unwrap();

    let mut walker = repo_writer.index_repo_with_args(|args| { args.fatal_missing_spans = true; });
    walker.follow_token("mod.cpp");
    walker.follow_token("C");
    walker.follow_token("F");
    assert_eq!(walker.node().text, "class F { C c; }");
}


#[test]
fn templated_member() {
    let mut repo_writer = RepoWriter::new(&testdir!());
    repo_writer.add_cpp_unit("mod.cpp", r#"
class F;

template <class X> class T {};

class C {
    T<F> t;
};
"#).unwrap();

    repo_writer.write_clang_compile_commands().unwrap();

    let mut walker = repo_writer.index_repo_with_args(|args| { args.fatal_missing_spans = true; });
    walker.follow_token("mod.cpp");
    walker.follow_token("C");
    walker.follow_token("T");
    assert_eq!(walker.node().text, "template <class X> class T {}");
}


#[test]
fn templated_member_external_definition_imported_in_namespace() {
    let mut repo_writer = RepoWriter::new(&testdir!());
    repo_writer.add("common.h", r#"
namespace N2 {
    template <class X> class T { int x; };
}
"#).unwrap();

    repo_writer.add("templ.h", r#"
namespace N2 {
    template <typename X> class T;
}

namespace N1 {
    using N2::T;
}
"#).unwrap();

    repo_writer.add_cpp_unit("mod.cpp", r#"
#include "common.h"
#include "templ.h"

using namespace N1;

class F;

class C {
    T<F> t;
};

"#).unwrap();

    repo_writer.write_clang_compile_commands().unwrap();

    let mut walker = repo_writer.index_repo_with_args(|args| { args.fatal_missing_spans = true; });
    walker.follow_token("mod.cpp");
    walker.follow_token("C");
    walker.follow_token("T");
    assert_eq!(walker.node().text, "template <class X> class T { int x; }");
}


#[test]
#[ignore]
fn type_imported_in_namespace() {
    let mut repo_writer = RepoWriter::new(&testdir!());
    repo_writer.add("common.h", r#"
namespace N2 {
    typedef int I;
}
"#).unwrap();

    repo_writer.add("templ.h", r#"

namespace N1 {
    using N2::I;
}
"#).unwrap();

    repo_writer.add_cpp_unit("mod.cpp", r#"
#include "common.h"
#include "templ.h"

using namespace N1;

class C { I t; };
"#).unwrap();

    repo_writer.write_clang_compile_commands().unwrap();

    let mut walker = repo_writer.index_repo_with_args(|args| { args.fatal_missing_spans = true; });
    walker.follow_token("mod.cpp");
    walker.follow_token("C");
    walker.follow_token("I");
    assert_eq!(walker.node().text, "typedef integer I");
}


#[test]
fn classes_with_elisions_maintain_original_line_numbers() {
    let mut repo_writer = RepoWriter::new(&testdir!());
    repo_writer.add_cpp_unit("main.cpp", r"
class C {
    void f() {
        int x;
        int y;
        int z;
    }
};
").unwrap();
    repo_writer.write_clang_compile_commands().unwrap();

    let mut walker = repo_writer.index_repo();
    walker.follow_token("main.cpp");
    walker.follow_token("C");

    let tokens = pb_node_tokens(walker.node());
    let got = tokens
        .iter()
        .map(|tok| (tok.line, tok.text.as_ref()))
        .collect::<Vec<(u32, &str)>>();
    let expected = vec![
        (2, "class"), (2, " "), (2, "C"), (2, " "), (2, "{"),
        (2, "\n    "),
        (3, "void"), (3, " "), (3, "f"), (3, "("), (3, ")"), (3, " "), (3, "{"),
        (3, " … "),
        (3, "}"),
        (7, "\n"),
        (8, "}"),
    ];
    assert_eq!(got, expected, "tokens don't match: {}", repr_diff(&expected, &got));
}


#[test]
fn extern_class() {
    let mut repo_writer = RepoWriter::new(&testdir!());
    repo_writer.add_cpp_unit("a.cpp", r"
class B;

class A {
    B *b;
};
").unwrap();
    repo_writer.add_cpp_unit("b.cpp", r"
class B { };
").unwrap();
    repo_writer.write_clang_compile_commands().unwrap();

    let mut walker = repo_writer.index_repo();
    walker.follow_token("a.cpp");
    walker.follow_token("A");
    walker.follow_token("B");

    assert_eq!(walker.node().text, "class B { }");
}


#[test]
fn extern_static_method() {
    let mut repo_writer = RepoWriter::new(&testdir!());

    repo_writer.add("a.h", r"
class A {
public:
    static void f();
};
").unwrap();

    repo_writer.add_cpp_unit("main.cpp", r#"
#include "a.h"

int main() {
    A::f();
}
"#).unwrap();
    repo_writer.add_cpp_unit("a.cpp", r#"
#include "a.h"

void A::f() { return; }
"#).unwrap();
    repo_writer.write_clang_compile_commands().unwrap();

    let mut walker = repo_writer.index_repo();
    walker.follow_token("main.cpp");
    walker.follow_token("main");
    walker.follow_token("f");
    assert_eq!(walker.node().text, "void A::f() { return; }");
}


#[test]
fn template_default_arguments_file_entry() {
    let mut repo_writer = RepoWriter::new(&testdir!());
    repo_writer.add_cpp_unit("a.cpp",
r"template<typename U>
class B { U u; };

template< typename T = B<int> >
class A {
    T *t;
};
").unwrap();
    repo_writer.write_clang_compile_commands().unwrap();

    let mut walker = repo_writer.index_repo();
    walker.follow_token("a.cpp");
    assert_eq!(
        walker.node().text,
r"template<typename U>
class B { … };

template< typename T = B<int> >
class A { … };");
}


#[test]
fn template_class_container() {
    let mut repo_writer = RepoWriter::new(&testdir!());
    repo_writer.add_cpp_unit("a.cpp", r"
template<typename U>
class B { U x; }
").unwrap();
    repo_writer.write_clang_compile_commands().unwrap();

    let mut walker = repo_writer.index_repo();
    walker.follow_token("a.cpp");
    let file_node_id = walker.node().id;

    walker.follow_token("B");
    assert_eq!(walker.node().container, Some(file_node_id));
}
