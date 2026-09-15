let output = "crates/rx82/notes/states.dot"
let header = r#'strict digraph CPU {
    // Graph styling
    bgcolor="transparent";
    pad=0.2;
    nodesep=0.1;

    // Global element styling
    node [
        fontname="Helvetica"
        style=filled
        fillcolor=lightblue
    ];

    edge [
        fontname="Courier New"
        color="#020202"
        arrowsize=0.8
        penwidth=1.4
    ];

    // Special node definitions
    FetchOpcode;
    RESET [shape=box fontcolor=white fillcolor=black];
    Decode;
    Execute [fillcolor=orange margin=0.3 width=5.0];

    // Edge definitions
    RESET -> WaitVecLo;
    Execute -> WaitInc     [label="inc (NN)"];
    Execute -> WaitDec     [label="dec (NN)"];
    Execute -> WaitData    [label=" ld R, (RR)\npop R"];
    Execute -> WaitCall    [label="call NN"];
    Execute -> WaitPS      [label="pop ps"];
    Execute -> PushData    [label="push R"];
    Execute -> WaitRetLo   [label="ret"];
    Execute -> WaitFlags   [label="rti"];
    Execute -> WaitStackHi [label="pop RR"];
    Execute -> PushRetLo   [label="trap T"];
'#
echo $header | save -f $output
cargo test --features states -- --show-output | lines | where $it =~ "->" | uniq | sort | save -f $output --append
echo "}" | save -f $output --append
