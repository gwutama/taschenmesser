@0xe353a58aa853fe2a;

interface UnitMan {
    getUnits @0 (wtf :Text) -> (units: List(Unit));
}

struct Unit {
    name @0 :Text;
    executable @1 :Text;
    arguments @2 :List(Text);
    dependencies @3 :List(Unit);
    restartPolicy @4 :RestartPolicy;
    uid @5 :Int32;
    gid @6 :Int32;
    enabled @7 :Bool;
    probeState @8 :ProbeState;
    pid @9 :Int32;
}

enum RestartPolicy {
    always @0;
    never @1;
}

enum ProbeState {
    undefined @0;
    alive @1;
    dead @2;
}

