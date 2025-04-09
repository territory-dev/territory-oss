use std::{collections::HashMap, path::PathBuf};

use serde::{Serialize, Deserialize};

use territory_core::{
    AbsolutePath, GToken, Location, NodeKind, Offset, RelativePath
};


pub type TransportID = u64;


#[derive(Serialize, Deserialize, Debug, PartialEq, Eq)]
pub struct ClangCommand {
    pub index: u64,
    pub file: PathBuf,
    pub directory: PathBuf,
    pub args: Vec<String>,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct Block {
    pub transport_key: TransportID,

    pub kind: NodeKind,
    pub member_of: Option<String>,
    pub start: Location,
    pub end: Location,

    pub sems: HashMap<TransportID, Sem>,
    pub text: Vec<GToken<ClangTokenContext>>,

    #[serde(flatten)]
    pub context: ClangNodeContext,
}


#[derive(Serialize, Deserialize, Debug)]
pub enum ClangTokenContext {
    Token {
        sem: Option<TransportID>,
        start: Location,
        end: Location,
    },
    Whitespace { text: String },
    Elided {
        start_offset: Offset,
        end_offset: Offset,
        nested_block_key: TransportID,
    },
}

#[derive(Serialize, Deserialize, Debug)]
pub struct ClangNodeContext {
    pub abs_path: AbsolutePath,
    pub relative_path: RelativePath,
    pub root: Sem,
    pub end_offset: Option<Offset>,
    // pub pre_comment: Vec<clang::token::Token<'a>>,
    pub nested: Option<Vec<(Location, Location, TransportID)>>,
    pub nest_level: usize,
    pub is_forward_decl: bool,
}


#[repr(u64)]
#[derive(Serialize, Deserialize, Debug, Clone, Copy, PartialEq, Eq)]
pub enum ClangCurKind {
    Unknown = 0,

    // IMPORTANT: If you add variants, update the from_raw() code below.
    /// A declaration whose specific type is not exposed via this interface.
    UnexposedDecl = 1,
    /// A C or C++ struct.
    StructDecl = 2,
    /// A C or C++ union.
    UnionDecl = 3,
    /// A C++ class.
    ClassDecl = 4,
    /// An enum.
    EnumDecl = 5,
    /// A C field or C++ non-static data member in a struct, union, or class.
    FieldDecl = 6,
    /// An enum constant.
    EnumConstantDecl = 7,
    /// A function.
    FunctionDecl = 8,
    /// A variable.
    VarDecl = 9,
    /// A parameter.
    ParmDecl = 10,
    /// An Objective-C `@interface`.
    ObjCInterfaceDecl = 11,
    /// An Objective-C `@interface` for a category.
    ObjCCategoryDecl = 12,
    /// An Objective-C `@protocol` declaration.
    ObjCProtocolDecl = 13,
    /// An Objective-C `@property` declaration.
    ObjCPropertyDecl = 14,
    /// An Objective-C instance variable.
    ObjCIvarDecl = 15,
    /// An Objective-C instance method.
    ObjCInstanceMethodDecl = 16,
    /// An Objective-C class method.
    ObjCClassMethodDecl = 17,
    /// An Objective-C `@implementation`.
    ObjCImplementationDecl = 18,
    /// An Objective-C `@implementation` for a category.
    ObjCCategoryImplDecl = 19,
    /// A typedef.
    TypedefDecl = 20,
    /// A C++ method.
    Method = 21,
    /// A C++ namespace.
    Namespace = 22,
    /// A linkage specification (e.g., `extern "C"`).
    LinkageSpec = 23,
    /// A C++ constructor.
    Constructor = 24,
    /// A C++ destructor.
    Destructor = 25,
    /// A C++ conversion function.
    ConversionFunction = 26,
    /// A C++ template type parameter.
    TemplateTypeParameter = 27,
    /// A C++ template non-type parameter.
    NonTypeTemplateParameter = 28,
    /// A C++ template template parameter.
    TemplateTemplateParameter = 29,
    /// A C++ function template.
    FunctionTemplate = 30,
    /// A C++ class template.
    ClassTemplate = 31,
    /// A C++ class template partial specialization.
    ClassTemplatePartialSpecialization = 32,
    /// A C++ namespace alias declaration.
    NamespaceAlias = 33,
    /// A C++ using directive.
    UsingDirective = 34,
    /// A C++ using declaration.
    UsingDeclaration = 35,
    /// A C++ type alias declaration.
    TypeAliasDecl = 36,
    /// An Objective-C `@synthesize` definition.
    ObjCSynthesizeDecl = 37,
    /// An Objective-C `@dynamic` definition.
    ObjCDynamicDecl = 38,
    /// An access specifier.
    AccessSpecifier = 39,
    /// A reference to a super class in Objective-C.
    ObjCSuperClassRef = 40,
    /// A reference to a protocol in Objective-C.
    ObjCProtocolRef = 41,
    /// A reference to a class in Objective-C.
    ObjCClassRef = 42,
    /// A reference to a type declaration.
    TypeRef = 43,
    /// A base class specifier.
    BaseSpecifier = 44,
    /// A reference to a class template, function template, template template parameter, or class
    /// template partial specialization.
    TemplateRef = 45,
    /// A reference to a namespace or namespace alias.
    NamespaceRef = 46,
    /// A reference to a member of a struct, union, or class that occurs in some non-expression
    /// context.
    MemberRef = 47,
    /// A reference to a labeled statement.
    LabelRef = 48,
    /// A reference to a set of overloaded functions or function templates that has not yet been
    /// resolved to a specific function or function template.
    OverloadedDeclRef = 49,
    /// A reference to a variable that occurs in some non-expression context.
    VariableRef = 50,
    /// Error: An invalid file.
    InvalidFile = 70,
    /// Error: An invalid decl which could not be found.
    InvalidDecl = 71,
    /// Error: An entity which is not yet supported by libclang, or this wrapper.
    NotImplemented = 72,
    /// Error: Invalid code.
    InvalidCode = 73,
    /// An expression whose specific kind is not exposed via this interface.
    UnexposedExpr = 100,
    /// An expression that refers to some value declaration, such as a function or enumerator.
    DeclRefExpr = 101,
    /// An expression that refers to the member of a struct, union, or class.
    MemberRefExpr = 102,
    /// An expression that calls a function.
    CallExpr = 103,
    /// An expression that sends a message to an Objective-C object or class.
    ObjCMessageExpr = 104,
    /// An expression that represents a block literal.
    BlockExpr = 105,
    /// An integer literal.
    IntegerLiteral = 106,
    /// A floating point number literal.
    FloatingLiteral = 107,
    /// An imaginary number literal.
    ImaginaryLiteral = 108,
    /// A string literal.
    StringLiteral = 109,
    /// A character literal.
    CharacterLiteral = 110,
    /// A parenthesized expression.
    ParenExpr = 111,
    /// Any unary expression other than `sizeof` and `alignof`.
    UnaryOperator = 112,
    /// An array subscript expression (`[C99 6.5.2.1]`).
    ArraySubscriptExpr = 113,
    /// A built-in binary expression (e.g., `x + y`).
    BinaryOperator = 114,
    /// A compound assignment expression (e.g., `x += y`).
    CompoundAssignOperator = 115,
    /// A ternary expression.
    ConditionalOperator = 116,
    /// An explicit cast in C or a C-style cast in C++.
    CStyleCastExpr = 117,
    /// A compound literal expression (`[C99 6.5.2.5]`).
    CompoundLiteralExpr = 118,
    /// A C or C++ initializer list.
    InitListExpr = 119,
    /// A GNU address of label expression.
    AddrLabelExpr = 120,
    /// A GNU statement expression.
    StmtExpr = 121,
    /// A C11 generic selection expression.
    GenericSelectionExpr = 122,
    /// A GNU `__null` expression.
    GNUNullExpr = 123,
    /// A C++ `static_cast<>` expression.
    StaticCastExpr = 124,
    /// A C++ `dynamic_cast<>` expression.
    DynamicCastExpr = 125,
    /// A C++ `reinterpret_cast<>` expression.
    ReinterpretCastExpr = 126,
    /// A C++ `const_cast<>` expression.
    ConstCastExpr = 127,
    /// A C++ cast that uses "function" notation (e.g., `int(0.5)`).
    FunctionalCastExpr = 128,
    /// A C++ `typeid` expression.
    TypeidExpr = 129,
    /// A C++ boolean literal.
    BoolLiteralExpr = 130,
    /// A C++ `nullptr` expression.
    NullPtrLiteralExpr = 131,
    /// A C++ `this` expression.
    ThisExpr = 132,
    /// A C++ `throw` expression.
    ThrowExpr = 133,
    /// A C++ `new` expression.
    NewExpr = 134,
    /// A C++ `delete` expression.
    DeleteExpr = 135,
    /// A unary expression.
    UnaryExpr = 136,
    /// An Objective-C string literal.
    ObjCStringLiteral = 137,
    /// An Objective-C `@encode` expression.
    ObjCEncodeExpr = 138,
    /// An Objective-C `@selector` expression.
    ObjCSelectorExpr = 139,
    /// An Objective-C `@protocol` expression.
    ObjCProtocolExpr = 140,
    /// An Objective-C bridged cast expression.
    ObjCBridgedCastExpr = 141,
    /// A C++11 parameter pack expansion expression.
    PackExpansionExpr = 142,
    /// A C++11 `sizeof...` expression.
    SizeOfPackExpr = 143,
    /// A C++11 lambda expression.
    LambdaExpr = 144,
    /// An Objective-C boolean literal.
    ObjCBoolLiteralExpr = 145,
    /// An Objective-C `self` expression.
    ObjCSelfExpr = 146,
    /// An OpenMP array section expression.
    ///
    /// Only produced by `libclang` 3.8 and later.
    OmpArraySectionExpr = 147,
    /// An Objective-C availability check expression (e.g., `@available(macos 10.10, *)`).
    ///
    /// Only produced by `libclang` 3.9 and later.
    ObjCAvailabilityCheckExpr = 148,
    /// A fixed-point literal.
    ///
    /// Only produced by `libclang` 7.0 and later.
    FixedPointLiteral = 149,
    /// A statement whose specific kind is not exposed via this interface.
    UnexposedStmt = 200,
    /// A labelled statement in a function.
    LabelStmt = 201,
    /// A group of statements (e.g., a function body).
    CompoundStmt = 202,
    /// A `case` statement.
    CaseStmt = 203,
    /// A `default` statement.
    DefaultStmt = 204,
    /// An `if` statement.
    IfStmt = 205,
    /// A `switch` statement.
    SwitchStmt = 206,
    /// A `while` statement.
    WhileStmt = 207,
    /// A `do` statement.
    DoStmt = 208,
    /// A `for` statement.
    ForStmt = 209,
    /// A `goto` statement.
    GotoStmt = 210,
    /// An indirect `goto` statement.
    IndirectGotoStmt = 211,
    /// A `continue` statement.
    ContinueStmt = 212,
    /// A `break` statement.
    BreakStmt = 213,
    /// A `return` statement.
    ReturnStmt = 214,
    /// An inline assembly statement.
    AsmStmt = 215,
    /// An Objective-C `@try`-`@catch`-`@finally` statement.
    ObjCAtTryStmt = 216,
    /// An Objective-C `@catch` statement.
    ObjCAtCatchStmt = 217,
    /// An Objective-C `@finally` statement.
    ObjCAtFinallyStmt = 218,
    /// An Objective-C `@throw` statement.
    ObjCAtThrowStmt = 219,
    /// An Objective-C `@synchronized` statement.
    ObjCAtSynchronizedStmt = 220,
    /// An Objective-C autorelease pool statement.
    ObjCAutoreleasePoolStmt = 221,
    /// An Objective-C collection statement.
    ObjCForCollectionStmt = 222,
    /// A C++ catch statement.
    CatchStmt = 223,
    /// A C++ try statement.
    TryStmt = 224,
    /// A C++11 range-based for statement.
    ForRangeStmt = 225,
    /// A Windows Structured Exception Handling `__try` statement.
    SehTryStmt = 226,
    /// A Windows Structured Exception Handling `__except` statement.
    SehExceptStmt = 227,
    /// A Windows Structured Exception Handling `__finally` statement.
    SehFinallyStmt = 228,
    /// A Windows Structured Exception Handling `__leave` statement.
    SehLeaveStmt = 247,
    /// A Microsoft inline assembly statement.
    MsAsmStmt = 229,
    /// A null statement.
    NullStmt = 230,
    /// An adaptor for mixing declarations with statements and expressions.
    DeclStmt = 231,
    /// An OpenMP parallel directive.
    OmpParallelDirective = 232,
    /// An OpenMP SIMD directive.
    OmpSimdDirective = 233,
    /// An OpenMP for directive.
    OmpForDirective = 234,
    /// An OpenMP sections directive.
    OmpSectionsDirective = 235,
    /// An OpenMP section directive.
    OmpSectionDirective = 236,
    /// An OpenMP single directive.
    OmpSingleDirective = 237,
    /// An OpenMP parallel for directive.
    OmpParallelForDirective = 238,
    /// An OpenMP parallel sections directive.
    OmpParallelSectionsDirective = 239,
    /// An OpenMP task directive.
    OmpTaskDirective = 240,
    /// An OpenMP master directive.
    OmpMasterDirective = 241,
    /// An OpenMP critical directive.
    OmpCriticalDirective = 242,
    /// An OpenMP taskyield directive.
    OmpTaskyieldDirective = 243,
    /// An OpenMP barrier directive.
    OmpBarrierDirective = 244,
    /// An OpenMP taskwait directive.
    OmpTaskwaitDirective = 245,
    /// An OpenMP flush directive.
    OmpFlushDirective = 246,
    /// An OpenMP ordered directive.
    ///
    /// Only produced by `libclang` 3.6 and later.
    OmpOrderedDirective = 248,
    /// An OpenMP atomic directive.
    ///
    /// Only produced by `libclang` 3.6 and later.
    OmpAtomicDirective = 249,
    /// An OpenMP for SIMD directive.
    ///
    /// Only produced by `libclang` 3.6 and later.
    OmpForSimdDirective = 250,
    /// An OpenMP parallel for SIMD directive.
    ///
    /// Only produced by `libclang` 3.6 and later.
    OmpParallelForSimdDirective = 251,
    /// An OpenMP target directive.
    ///
    /// Only produced by `libclang` 3.6 and later.
    OmpTargetDirective = 252,
    /// An OpenMP teams directive.
    ///
    /// Only produced by `libclang` 3.6 and later.
    OmpTeamsDirective = 253,
    /// An OpenMP taskgroup directive.
    ///
    /// Only produced by `libclang` 3.7 and later.
    OmpTaskgroupDirective = 254,
    /// An OpenMP cancellation point directive.
    ///
    /// Only produced by `libclang` 3.7 and later.
    OmpCancellationPointDirective = 255,
    /// An OpenMP cancel directive.
    ///
    /// Only produced by `libclang` 3.7 and later.
    OmpCancelDirective = 256,
    /// An OpenMP target data directive.
    ///
    /// Only produced by `libclang` 3.8 and later.
    OmpTargetDataDirective = 257,
    /// An OpenMP task loop directive.
    ///
    /// Only produced by `libclang` 3.8 and later.
    OmpTaskLoopDirective = 258,
    /// An OpenMP task loop SIMD directive.
    ///
    /// Only produced by `libclang` 3.8 and later.
    OmpTaskLoopSimdDirective = 259,
    /// An OpenMP distribute directive.
    ///
    /// Only produced by `libclang` 3.8 and later.
    OmpDistributeDirective = 260,
    /// An OpenMP target enter data directive.
    ///
    /// Only produced by `libclang` 3.9 and later.
    OmpTargetEnterDataDirective = 261,
    /// An OpenMP target exit data directive.
    ///
    /// Only produced by `libclang` 3.9 and later.
    OmpTargetExitDataDirective = 262,
    /// An OpenMP target parallel directive.
    ///
    /// Only produced by `libclang` 3.9 and later.
    OmpTargetParallelDirective = 263,
    /// An OpenMP target parallel for directive.
    ///
    /// Only produced by `libclang` 3.9 and later.
    OmpTargetParallelForDirective = 264,
    /// An OpenMP target update directive.
    ///
    /// Only produced by `libclang` 3.9 and later.
    OmpTargetUpdateDirective = 265,
    /// An OpenMP distribute parallel for directive.
    ///
    /// Only produced by `libclang` 3.9 and later.
    OmpDistributeParallelForDirective = 266,
    /// An OpenMP distribute parallel for SIMD directive.
    ///
    /// Only produced by `libclang` 3.9 and later.
    OmpDistributeParallelForSimdDirective = 267,
    /// An OpenMP distribute SIMD directive.
    ///
    /// Only produced by `libclang` 3.9 and later.
    OmpDistributeSimdDirective = 268,
    /// An OpenMP target parallel for SIMD directive.
    ///
    /// Only produced by `libclang` 3.9 and later.
    OmpTargetParallelForSimdDirective = 269,
    /// An OpenMP target SIMD directive.
    ///
    /// Only produced by `libclang` 4.0 and later.
    OmpTargetSimdDirective = 270,
    /// An OpenMP teams distribute directive.
    ///
    /// Only produced by `libclang` 4.0 and later.
    OmpTeamsDistributeDirective = 271,
    /// An OpenMP teams distribute SIMD directive.
    ///
    /// Only produced by `libclang` 4.0 and later.
    OmpTeamsDistributeSimdDirective = 272,
    /// An OpenMP teams distribute parallel for SIMD directive.
    ///
    /// Only produced by `libclang` 4.0 and later.
    OmpTeamsDistributeParallelForSimdDirective = 273,
    /// An OpenMP teams distribute parallel for directive.
    ///
    /// Only produced by `libclang` 4.0 and later.
    OmpTeamsDistributeParallelForDirective = 274,
    /// An OpenMP target teams directive.
    ///
    /// Only produced by `libclang` 4.0 and later.
    OmpTargetTeamsDirective = 275,
    /// An OpenMP target teams distribute directive.
    ///
    /// Only produced by `libclang` 4.0 and later.
    OmpTargetTeamsDistributeDirective = 276,
    /// An OpenMP target teams distribute parallel for directive.
    ///
    /// Only produced by `libclang` 4.0 and later.
    OmpTargetTeamsDistributeParallelForDirective = 277,
    /// An OpenMP target teams distribute parallel for SIMD directive.
    ///
    /// Only produced by `libclang` 4.0 and later.
    OmpTargetTeamsDistributeParallelForSimdDirective = 278,
    /// An OpenMP target teams distribute SIMD directive.
    ///
    /// Only produced by `libclang` 4.0 and later.
    OmpTargetTeamsDistributeSimdDirective = 279,
    /// C++2a std::bit_cast expression.
    ///
    /// Only produced by 'libclang' 9.0 and later.
    BitCastExpr = 280,
    /// An OpenMP master task loop directive.
    ///
    /// Only produced by `libclang` 10.0 and later.
    OmpMasterTaskLoopDirective = 281,
    /// An OpenMP parallel master task loop directive.
    ///
    /// Only produced by `libclang` 10.0 and later.
    OmpParallelMasterTaskLoopDirective = 282,
    /// An OpenMP master task loop SIMD directive.
    ///
    /// Only produced by `libclang` 10.0 and later.
    OmpMasterTaskLoopSimdDirective = 283,
    /// An OpenMP parallel master task loop SIMD directive.
    ///
    /// Only produced by `libclang` 10.0 and later.
    OmpParallelMasterTaskLoopSimdDirective = 284,
    /// An OpenMP parallel master directive.
    ///
    /// Only produced by `libclang` 10.0 and later.
    OmpParallelMasterDirective = 285,
    /// The top-level AST entity which acts as the root for the other entitys.
    TranslationUnit = 300,
    /// An attribute whose specific kind is not exposed via this interface.
    UnexposedAttr = 400,
    /// An attribute applied to an Objective-C IBAction.
    IbActionAttr = 401,
    /// An attribute applied to an Objective-C IBOutlet.
    IbOutletAttr = 402,
    /// An attribute applied to an Objective-C IBOutletCollection.
    IbOutletCollectionAttr = 403,
    /// The `final` attribute.
    FinalAttr = 404,
    /// The `override` attribute.
    OverrideAttr = 405,
    /// An annotation attribute.
    AnnotateAttr = 406,
    /// An ASM label attribute.
    AsmLabelAttr = 407,
    /// An attribute that requests for packed records (e.g., `__attribute__ ((__packed__))`).
    PackedAttr = 408,
    /// An attribute that asserts a function has no side effects (e.g., `__attribute__((pure))`).
    PureAttr = 409,
    /// The `const` attribute.
    ConstAttr = 410,
    /// An attribute that allows calls to a function to be duplicated by the optimized
    /// (e.g., `__attribute__((noduplicate))`).
    NoDuplicateAttr = 411,
    /// A CUDA constant attribute.
    CudaConstantAttr = 412,
    /// A CUDA device attribute.
    CudaDeviceAttr = 413,
    /// A CUDA global attribute.
    CudaGlobalAttr = 414,
    /// A CUDA host attribute.
    CudaHostAttr = 415,
    /// A CUDA shared attribute.
    ///
    /// Only produced by `libclang` 3.6 and later.
    CudaSharedAttr = 416,
    /// A linker visibility attribute.
    ///
    /// Only produced by `libclang` 3.8 and later.
    VisibilityAttr = 417,
    /// A MSVC DLL export attribute.
    ///
    /// Only produced by `libclang` 3.8 and later.
    DllExport = 418,
    /// A MSVC DLL import attribute.
    ///
    /// Only produced by `libclang` 3.8 and later.
    DllImport = 419,
    /// `__attribute__((ns_returns_retained))`
    ///
    /// Only produced by `libclang` 8.0 and later.
    NSReturnsRetained = 420,
    /// `__attribute__((ns_returns_not_retained))`
    ///
    /// Only produced by `libclang` 8.0 and later.
    NSReturnsNotRetained = 421,
    /// `__attribute__((ns_returns_autoreleased))`
    ///
    /// Only produced by `libclang` 8.0 and later.
    NSReturnsAutoreleased = 422,
    /// `__attribute__((ns_consumes_self))`
    ///
    /// Only produced by `libclang` 8.0 and later.
    NSConsumesSelf = 423,
    /// `__attribute__((ns_consumed))`
    ///
    /// Only produced by `libclang` 8.0 and later.
    NSConsumed = 424,
    /// `__attribute__((objc_exception))`
    ///
    /// Only produced by `libclang` 8.0 and later.
    ObjCException = 425,
    /// `__attribute__((NSObject))`
    ///
    /// Only produced by `libclang` 8.0 and later.
    ObjCNSObject = 426,
    /// `__attribute__((objc_independent_class))`
    ///
    /// Only produced by `libclang` 8.0 and later.
    ObjCIndependentClass = 427,
    /// `__attribute__((objc_precise_lifetime))`
    ///
    /// Only produced by `libclang` 8.0 and later.
    ObjCPreciseLifetime = 428,
    /// `__attribute__((objc_returns_inner_pointer))`
    ///
    /// Only produced by `libclang` 8.0 and later.
    ObjCReturnsInnerPointer = 429,
    /// `__attribute__((objc_requires_super))`
    ///
    /// Only produced by `libclang` 8.0 and later.
    ObjCRequiresSuper = 430,
    /// `__attribute__((objc_root_class))`
    ///
    /// Only produced by `libclang` 8.0 and later.
    ObjCRootClass = 431,
    /// `__attribute__((objc_subclassing_restricted))`
    ///
    /// Only produced by `libclang` 8.0 and later.
    ObjCSubclassingRestricted = 432,
    /// `__attribute__((objc_protocol_requires_explicit_implementation))`
    ///
    /// Only produced by `libclang` 8.0 and later.
    ObjCExplicitProtocolImpl = 433,
    /// `__attribute__((objc_designated_initializer))`
    ///
    /// Only produced by `libclang` 8.0 and later.
    ObjCDesignatedInitializer = 434,
    /// `__attribute__((objc_runtime_visible))`
    ///
    /// Only produced by `libclang` 8.0 and later.
    ObjCRuntimeVisible = 435,
    /// `__attribute__((objc_boxable))`
    ///
    /// Only produced by `libclang` 8.0 and later.
    ObjCBoxable = 436,
    /// `__attribute__((flag_enum))`
    ///
    /// Only produced by `libclang` 8.0 and later.
    FlagEnum = 437,
    /// `__attribute__((clang::convergent))`
    ///
    /// Only produced by `libclang` 9.0 and later.
    ConvergentAttr  = 438,
    /// Only produced by `libclang` 9.0 and later.
    WarnUnusedAttr = 439,
    /// `__attribute__((nodiscard))`
    ///
    /// Only produced by `libclang` 9.0 and later.
    WarnUnusedResultAttr = 440,
    /// Only produced by `libclang` 9.0 and later.
    AlignedAttr = 441,
    /// A preprocessing directive.
    PreprocessingDirective = 500,
    /// A macro definition.
    MacroDefinition = 501,
    /// A macro expansion.
    MacroExpansion = 502,
    /// An inclusion directive.
    InclusionDirective = 503,
    /// A module import declaration.
    ModuleImportDecl = 600,
    /// A C++11 alias template declaration (e.g., `template <typename T> using M = std::map<T, T>`).
    ///
    /// Only produced by `libclang` 3.8 and later.
    TypeAliasTemplateDecl = 601,
    /// A `static_assert` node.
    ///
    /// Only produced by `libclang` 3.9 and later.
    StaticAssert = 602,
    /// A friend declaration.
    ///
    /// Only produced by `libclang` 4.0 and later.
    FriendDecl = 603,
    /// A single overload in a set of overloads.
    ///
    /// Only produced by `libclang` 3.7 and later.
    OverloadCandidate = 700,
}


#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LocalDefinitionLocation {
    pub path: RelativePath,
    pub offset: Offset,
    pub curloc: String,
}


#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Sem {
    #[serde(rename="i")]
    pub transport_id: u64,

    #[serde(rename="u")]
    pub usr: Option<String>,
    #[serde(rename="k")]
    pub kind: ClangCurKind,
    #[serde(rename="n")]
    pub name: Option<String>,
    #[serde(rename="l")]
    pub linkage: Option<ClangLinkage>,
    #[serde(rename="de")]
    pub is_definition: bool,
    #[serde(rename="dc")]
    pub is_declaration: bool,
    #[serde(rename="fm")]
    pub is_function_like_macro: bool,
    #[serde(rename="d")]
    pub local_defintion: Option<LocalDefinitionLocation>,
    #[serde(rename="t")]
    pub type_: Option<String>,
    #[serde(rename="cs")]
    pub cur_start_offset: Option<Offset>,
    #[serde(rename="ce")]
    pub cur_end_offset: Option<Offset>,
    #[serde(rename="c")]
    pub definition_context: Vec<String>,
    #[serde(rename="N")]
    pub display_name: Option<String>,
    #[serde(rename="C")]
    pub curloc: String,
}


impl From<clang::EntityKind> for ClangCurKind {
    fn from(value: clang::EntityKind) -> Self {
        match value {
            clang::EntityKind::UnexposedDecl => Self::UnexposedDecl,
            clang::EntityKind::StructDecl => Self::StructDecl,
            clang::EntityKind::UnionDecl => Self::UnionDecl,
            clang::EntityKind::ClassDecl => Self::ClassDecl,
            clang::EntityKind::EnumDecl => Self::EnumDecl,
            clang::EntityKind::FieldDecl => Self::FieldDecl,
            clang::EntityKind::EnumConstantDecl => Self::EnumConstantDecl,
            clang::EntityKind::FunctionDecl => Self::FunctionDecl,
            clang::EntityKind::VarDecl => Self::VarDecl,
            clang::EntityKind::ParmDecl => Self::ParmDecl,
            clang::EntityKind::ObjCInterfaceDecl => Self::ObjCInterfaceDecl,
            clang::EntityKind::ObjCCategoryDecl => Self::ObjCCategoryDecl,
            clang::EntityKind::ObjCProtocolDecl => Self::ObjCProtocolDecl,
            clang::EntityKind::ObjCPropertyDecl => Self::ObjCPropertyDecl,
            clang::EntityKind::ObjCIvarDecl => Self::ObjCIvarDecl,
            clang::EntityKind::ObjCInstanceMethodDecl => Self::ObjCInstanceMethodDecl,
            clang::EntityKind::ObjCClassMethodDecl => Self::ObjCClassMethodDecl,
            clang::EntityKind::ObjCImplementationDecl => Self::ObjCImplementationDecl,
            clang::EntityKind::ObjCCategoryImplDecl => Self::ObjCCategoryImplDecl,
            clang::EntityKind::TypedefDecl => Self::TypedefDecl,
            clang::EntityKind::Method => Self::Method,
            clang::EntityKind::Namespace => Self::Namespace,
            clang::EntityKind::LinkageSpec => Self::LinkageSpec,
            clang::EntityKind::Constructor => Self::Constructor,
            clang::EntityKind::Destructor => Self::Destructor,
            clang::EntityKind::ConversionFunction => Self::ConversionFunction,
            clang::EntityKind::TemplateTypeParameter => Self::TemplateTypeParameter,
            clang::EntityKind::NonTypeTemplateParameter => Self::NonTypeTemplateParameter,
            clang::EntityKind::TemplateTemplateParameter => Self::TemplateTemplateParameter,
            clang::EntityKind::FunctionTemplate => Self::FunctionTemplate,
            clang::EntityKind::ClassTemplate => Self::ClassTemplate,
            clang::EntityKind::ClassTemplatePartialSpecialization => Self::ClassTemplatePartialSpecialization,
            clang::EntityKind::NamespaceAlias => Self::NamespaceAlias,
            clang::EntityKind::UsingDirective => Self::UsingDirective,
            clang::EntityKind::UsingDeclaration => Self::UsingDeclaration,
            clang::EntityKind::TypeAliasDecl => Self::TypeAliasDecl,
            clang::EntityKind::ObjCSynthesizeDecl => Self::ObjCSynthesizeDecl,
            clang::EntityKind::ObjCDynamicDecl => Self::ObjCDynamicDecl,
            clang::EntityKind::AccessSpecifier => Self::AccessSpecifier,
            clang::EntityKind::ObjCSuperClassRef => Self::ObjCSuperClassRef,
            clang::EntityKind::ObjCProtocolRef => Self::ObjCProtocolRef,
            clang::EntityKind::ObjCClassRef => Self::ObjCClassRef,
            clang::EntityKind::TypeRef => Self::TypeRef,
            clang::EntityKind::BaseSpecifier => Self::BaseSpecifier,
            clang::EntityKind::TemplateRef => Self::TemplateRef,
            clang::EntityKind::NamespaceRef => Self::NamespaceRef,
            clang::EntityKind::MemberRef => Self::MemberRef,
            clang::EntityKind::LabelRef => Self::LabelRef,
            clang::EntityKind::OverloadedDeclRef => Self::OverloadedDeclRef,
            clang::EntityKind::VariableRef => Self::VariableRef,
            clang::EntityKind::InvalidFile => Self::InvalidFile,
            clang::EntityKind::InvalidDecl => Self::InvalidDecl,
            clang::EntityKind::NotImplemented => Self::NotImplemented,
            clang::EntityKind::InvalidCode => Self::InvalidCode,
            clang::EntityKind::UnexposedExpr => Self::UnexposedExpr,
            clang::EntityKind::DeclRefExpr => Self::DeclRefExpr,
            clang::EntityKind::MemberRefExpr => Self::MemberRefExpr,
            clang::EntityKind::CallExpr => Self::CallExpr,
            clang::EntityKind::ObjCMessageExpr => Self::ObjCMessageExpr,
            clang::EntityKind::BlockExpr => Self::BlockExpr,
            clang::EntityKind::IntegerLiteral => Self::IntegerLiteral,
            clang::EntityKind::FloatingLiteral => Self::FloatingLiteral,
            clang::EntityKind::ImaginaryLiteral => Self::ImaginaryLiteral,
            clang::EntityKind::StringLiteral => Self::StringLiteral,
            clang::EntityKind::CharacterLiteral => Self::CharacterLiteral,
            clang::EntityKind::ParenExpr => Self::ParenExpr,
            clang::EntityKind::UnaryOperator => Self::UnaryOperator,
            clang::EntityKind::ArraySubscriptExpr => Self::ArraySubscriptExpr,
            clang::EntityKind::BinaryOperator => Self::BinaryOperator,
            clang::EntityKind::CompoundAssignOperator => Self::CompoundAssignOperator,
            clang::EntityKind::ConditionalOperator => Self::ConditionalOperator,
            clang::EntityKind::CStyleCastExpr => Self::CStyleCastExpr,
            clang::EntityKind::CompoundLiteralExpr => Self::CompoundLiteralExpr,
            clang::EntityKind::InitListExpr => Self::InitListExpr,
            clang::EntityKind::AddrLabelExpr => Self::AddrLabelExpr,
            clang::EntityKind::StmtExpr => Self::StmtExpr,
            clang::EntityKind::GenericSelectionExpr => Self::GenericSelectionExpr,
            clang::EntityKind::GNUNullExpr => Self::GNUNullExpr,
            clang::EntityKind::StaticCastExpr => Self::StaticCastExpr,
            clang::EntityKind::DynamicCastExpr => Self::DynamicCastExpr,
            clang::EntityKind::ReinterpretCastExpr => Self::ReinterpretCastExpr,
            clang::EntityKind::ConstCastExpr => Self::ConstCastExpr,
            clang::EntityKind::FunctionalCastExpr => Self::FunctionalCastExpr,
            clang::EntityKind::TypeidExpr => Self::TypeidExpr,
            clang::EntityKind::BoolLiteralExpr => Self::BoolLiteralExpr,
            clang::EntityKind::NullPtrLiteralExpr => Self::NullPtrLiteralExpr,
            clang::EntityKind::ThisExpr => Self::ThisExpr,
            clang::EntityKind::ThrowExpr => Self::ThrowExpr,
            clang::EntityKind::NewExpr => Self::NewExpr,
            clang::EntityKind::DeleteExpr => Self::DeleteExpr,
            clang::EntityKind::UnaryExpr => Self::UnaryExpr,
            clang::EntityKind::ObjCStringLiteral => Self::ObjCStringLiteral,
            clang::EntityKind::ObjCEncodeExpr => Self::ObjCEncodeExpr,
            clang::EntityKind::ObjCSelectorExpr => Self::ObjCSelectorExpr,
            clang::EntityKind::ObjCProtocolExpr => Self::ObjCProtocolExpr,
            clang::EntityKind::ObjCBridgedCastExpr => Self::ObjCBridgedCastExpr,
            clang::EntityKind::PackExpansionExpr => Self::PackExpansionExpr,
            clang::EntityKind::SizeOfPackExpr => Self::SizeOfPackExpr,
            clang::EntityKind::LambdaExpr => Self::LambdaExpr,
            clang::EntityKind::ObjCBoolLiteralExpr => Self::ObjCBoolLiteralExpr,
            clang::EntityKind::ObjCSelfExpr => Self::ObjCSelfExpr,
            clang::EntityKind::OmpArraySectionExpr => Self::OmpArraySectionExpr,
            clang::EntityKind::ObjCAvailabilityCheckExpr => Self::ObjCAvailabilityCheckExpr,
            clang::EntityKind::FixedPointLiteral => Self::FixedPointLiteral,
            clang::EntityKind::UnexposedStmt => Self::UnexposedStmt,
            clang::EntityKind::LabelStmt => Self::LabelStmt,
            clang::EntityKind::CompoundStmt => Self::CompoundStmt,
            clang::EntityKind::CaseStmt => Self::CaseStmt,
            clang::EntityKind::DefaultStmt => Self::DefaultStmt,
            clang::EntityKind::IfStmt => Self::IfStmt,
            clang::EntityKind::SwitchStmt => Self::SwitchStmt,
            clang::EntityKind::WhileStmt => Self::WhileStmt,
            clang::EntityKind::DoStmt => Self::DoStmt,
            clang::EntityKind::ForStmt => Self::ForStmt,
            clang::EntityKind::GotoStmt => Self::GotoStmt,
            clang::EntityKind::IndirectGotoStmt => Self::IndirectGotoStmt,
            clang::EntityKind::ContinueStmt => Self::ContinueStmt,
            clang::EntityKind::BreakStmt => Self::BreakStmt,
            clang::EntityKind::ReturnStmt => Self::ReturnStmt,
            clang::EntityKind::AsmStmt => Self::AsmStmt,
            clang::EntityKind::ObjCAtTryStmt => Self::ObjCAtTryStmt,
            clang::EntityKind::ObjCAtCatchStmt => Self::ObjCAtCatchStmt,
            clang::EntityKind::ObjCAtFinallyStmt => Self::ObjCAtFinallyStmt,
            clang::EntityKind::ObjCAtThrowStmt => Self::ObjCAtThrowStmt,
            clang::EntityKind::ObjCAtSynchronizedStmt => Self::ObjCAtSynchronizedStmt,
            clang::EntityKind::ObjCAutoreleasePoolStmt => Self::ObjCAutoreleasePoolStmt,
            clang::EntityKind::ObjCForCollectionStmt => Self::ObjCForCollectionStmt,
            clang::EntityKind::CatchStmt => Self::CatchStmt,
            clang::EntityKind::TryStmt => Self::TryStmt,
            clang::EntityKind::ForRangeStmt => Self::ForRangeStmt,
            clang::EntityKind::SehTryStmt => Self::SehTryStmt,
            clang::EntityKind::SehExceptStmt => Self::SehExceptStmt,
            clang::EntityKind::SehFinallyStmt => Self::SehFinallyStmt,
            clang::EntityKind::SehLeaveStmt => Self::SehLeaveStmt,
            clang::EntityKind::MsAsmStmt => Self::MsAsmStmt,
            clang::EntityKind::NullStmt => Self::NullStmt,
            clang::EntityKind::DeclStmt => Self::DeclStmt,
            clang::EntityKind::OmpParallelDirective => Self::OmpParallelDirective,
            clang::EntityKind::OmpSimdDirective => Self::OmpSimdDirective,
            clang::EntityKind::OmpForDirective => Self::OmpForDirective,
            clang::EntityKind::OmpSectionsDirective => Self::OmpSectionsDirective,
            clang::EntityKind::OmpSectionDirective => Self::OmpSectionDirective,
            clang::EntityKind::OmpSingleDirective => Self::OmpSingleDirective,
            clang::EntityKind::OmpParallelForDirective => Self::OmpParallelForDirective,
            clang::EntityKind::OmpParallelSectionsDirective => Self::OmpParallelSectionsDirective,
            clang::EntityKind::OmpTaskDirective => Self::OmpTaskDirective,
            clang::EntityKind::OmpMasterDirective => Self::OmpMasterDirective,
            clang::EntityKind::OmpCriticalDirective => Self::OmpCriticalDirective,
            clang::EntityKind::OmpTaskyieldDirective => Self::OmpTaskyieldDirective,
            clang::EntityKind::OmpBarrierDirective => Self::OmpBarrierDirective,
            clang::EntityKind::OmpTaskwaitDirective => Self::OmpTaskwaitDirective,
            clang::EntityKind::OmpFlushDirective => Self::OmpFlushDirective,
            clang::EntityKind::OmpOrderedDirective => Self::OmpOrderedDirective,
            clang::EntityKind::OmpAtomicDirective => Self::OmpAtomicDirective,
            clang::EntityKind::OmpForSimdDirective => Self::OmpForSimdDirective,
            clang::EntityKind::OmpParallelForSimdDirective => Self::OmpParallelForSimdDirective,
            clang::EntityKind::OmpTargetDirective => Self::OmpTargetDirective,
            clang::EntityKind::OmpTeamsDirective => Self::OmpTeamsDirective,
            clang::EntityKind::OmpTaskgroupDirective => Self::OmpTaskgroupDirective,
            clang::EntityKind::OmpCancellationPointDirective => Self::OmpCancellationPointDirective,
            clang::EntityKind::OmpCancelDirective => Self::OmpCancelDirective,
            clang::EntityKind::OmpTargetDataDirective => Self::OmpTargetDataDirective,
            clang::EntityKind::OmpTaskLoopDirective => Self::OmpTaskLoopDirective,
            clang::EntityKind::OmpTaskLoopSimdDirective => Self::OmpTaskLoopSimdDirective,
            clang::EntityKind::OmpDistributeDirective => Self::OmpDistributeDirective,
            clang::EntityKind::OmpTargetEnterDataDirective => Self::OmpTargetEnterDataDirective,
            clang::EntityKind::OmpTargetExitDataDirective => Self::OmpTargetExitDataDirective,
            clang::EntityKind::OmpTargetParallelDirective => Self::OmpTargetParallelDirective,
            clang::EntityKind::OmpTargetParallelForDirective => Self::OmpTargetParallelForDirective,
            clang::EntityKind::OmpTargetUpdateDirective => Self::OmpTargetUpdateDirective,
            clang::EntityKind::OmpDistributeParallelForDirective => Self::OmpDistributeParallelForDirective,
            clang::EntityKind::OmpDistributeParallelForSimdDirective => Self::OmpDistributeParallelForSimdDirective,
            clang::EntityKind::OmpDistributeSimdDirective => Self::OmpDistributeSimdDirective,
            clang::EntityKind::OmpTargetParallelForSimdDirective => Self::OmpTargetParallelForSimdDirective,
            clang::EntityKind::OmpTargetSimdDirective => Self::OmpTargetSimdDirective,
            clang::EntityKind::OmpTeamsDistributeDirective => Self::OmpTeamsDistributeDirective,
            clang::EntityKind::OmpTeamsDistributeSimdDirective => Self::OmpTeamsDistributeSimdDirective,
            clang::EntityKind::OmpTeamsDistributeParallelForSimdDirective => Self::OmpTeamsDistributeParallelForSimdDirective,
            clang::EntityKind::OmpTeamsDistributeParallelForDirective => Self::OmpTeamsDistributeParallelForDirective,
            clang::EntityKind::OmpTargetTeamsDirective => Self::OmpTargetTeamsDirective,
            clang::EntityKind::OmpTargetTeamsDistributeDirective => Self::OmpTargetTeamsDistributeDirective,
            clang::EntityKind::OmpTargetTeamsDistributeParallelForDirective => Self::OmpTargetTeamsDistributeParallelForDirective,
            clang::EntityKind::OmpTargetTeamsDistributeParallelForSimdDirective => Self::OmpTargetTeamsDistributeParallelForSimdDirective,
            clang::EntityKind::OmpTargetTeamsDistributeSimdDirective => Self::OmpTargetTeamsDistributeSimdDirective,
            clang::EntityKind::BitCastExpr => Self::BitCastExpr,
            clang::EntityKind::OmpMasterTaskLoopDirective => Self::OmpMasterTaskLoopDirective,
            clang::EntityKind::OmpParallelMasterTaskLoopDirective => Self::OmpParallelMasterTaskLoopDirective,
            clang::EntityKind::OmpMasterTaskLoopSimdDirective => Self::OmpMasterTaskLoopSimdDirective,
            clang::EntityKind::OmpParallelMasterTaskLoopSimdDirective => Self::OmpParallelMasterTaskLoopSimdDirective,
            clang::EntityKind::OmpParallelMasterDirective => Self::OmpParallelMasterDirective,
            clang::EntityKind::TranslationUnit => Self::TranslationUnit,
            clang::EntityKind::UnexposedAttr => Self::UnexposedAttr,
            clang::EntityKind::IbActionAttr => Self::IbActionAttr,
            clang::EntityKind::IbOutletAttr => Self::IbOutletAttr,
            clang::EntityKind::IbOutletCollectionAttr => Self::IbOutletCollectionAttr,
            clang::EntityKind::FinalAttr => Self::FinalAttr,
            clang::EntityKind::OverrideAttr => Self::OverrideAttr,
            clang::EntityKind::AnnotateAttr => Self::AnnotateAttr,
            clang::EntityKind::AsmLabelAttr => Self::AsmLabelAttr,
            clang::EntityKind::PackedAttr => Self::PackedAttr,
            clang::EntityKind::PureAttr => Self::PureAttr,
            clang::EntityKind::ConstAttr => Self::ConstAttr,
            clang::EntityKind::NoDuplicateAttr => Self::NoDuplicateAttr,
            clang::EntityKind::CudaConstantAttr => Self::CudaConstantAttr,
            clang::EntityKind::CudaDeviceAttr => Self::CudaDeviceAttr,
            clang::EntityKind::CudaGlobalAttr => Self::CudaGlobalAttr,
            clang::EntityKind::CudaHostAttr => Self::CudaHostAttr,
            clang::EntityKind::CudaSharedAttr => Self::CudaSharedAttr,
            clang::EntityKind::VisibilityAttr => Self::VisibilityAttr,
            clang::EntityKind::DllExport => Self::DllExport,
            clang::EntityKind::DllImport => Self::DllImport,
            clang::EntityKind::NSReturnsRetained => Self::NSReturnsRetained,
            clang::EntityKind::NSReturnsNotRetained => Self::NSReturnsNotRetained,
            clang::EntityKind::NSReturnsAutoreleased => Self::NSReturnsAutoreleased,
            clang::EntityKind::NSConsumesSelf => Self::NSConsumesSelf,
            clang::EntityKind::NSConsumed => Self::NSConsumed,
            clang::EntityKind::ObjCException => Self::ObjCException,
            clang::EntityKind::ObjCNSObject => Self::ObjCNSObject,
            clang::EntityKind::ObjCIndependentClass => Self::ObjCIndependentClass,
            clang::EntityKind::ObjCPreciseLifetime => Self::ObjCPreciseLifetime,
            clang::EntityKind::ObjCReturnsInnerPointer => Self::ObjCReturnsInnerPointer,
            clang::EntityKind::ObjCRequiresSuper => Self::ObjCRequiresSuper,
            clang::EntityKind::ObjCRootClass => Self::ObjCRootClass,
            clang::EntityKind::ObjCSubclassingRestricted => Self::ObjCSubclassingRestricted,
            clang::EntityKind::ObjCExplicitProtocolImpl => Self::ObjCExplicitProtocolImpl,
            clang::EntityKind::ObjCDesignatedInitializer => Self::ObjCDesignatedInitializer,
            clang::EntityKind::ObjCRuntimeVisible => Self::ObjCRuntimeVisible,
            clang::EntityKind::ObjCBoxable => Self::ObjCBoxable,
            clang::EntityKind::FlagEnum => Self::FlagEnum,
            clang::EntityKind::ConvergentAttr  => Self::ConvergentAttr,
            clang::EntityKind::WarnUnusedAttr => Self::WarnUnusedAttr,
            clang::EntityKind::WarnUnusedResultAttr => Self::WarnUnusedResultAttr,
            clang::EntityKind::AlignedAttr => Self::AlignedAttr,
            clang::EntityKind::PreprocessingDirective => Self::PreprocessingDirective,
            clang::EntityKind::MacroDefinition => Self::MacroDefinition,
            clang::EntityKind::MacroExpansion => Self::MacroExpansion,
            clang::EntityKind::InclusionDirective => Self::InclusionDirective,
            clang::EntityKind::ModuleImportDecl => Self::ModuleImportDecl,
            clang::EntityKind::TypeAliasTemplateDecl => Self::TypeAliasTemplateDecl,
            clang::EntityKind::StaticAssert => Self::StaticAssert,
            clang::EntityKind::FriendDecl => Self::FriendDecl,
            clang::EntityKind::OverloadCandidate => Self::OverloadCandidate,
        }
    }
}


#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub enum ClangLinkage {
    Unknown,

    /// The AST entity has automatic storage (e.g., variables or parameters).
    Automatic = 1,
    /// The AST entity is a static variable or static function.
    Internal = 2,
    /// The AST entity has external linkage.
    External = 4,
    /// The AST entity has external linkage and lives in a C++ anonymous namespace.
    UniqueExternal = 3,
}


impl From<clang::Linkage> for ClangLinkage {
    fn from(value: clang::Linkage) -> Self {
        match value {
            clang::Linkage::Automatic => Self::Automatic,
            clang::Linkage::Internal => Self::Internal,
            clang::Linkage::External => Self::External,
            clang::Linkage::UniqueExternal => Self::UniqueExternal,
        }
    }
}
