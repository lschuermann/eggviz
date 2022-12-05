"use strict";

import 'materialize-css/dist/css/materialize.min.css';
import 'materialize-css/dist/js/materialize.min';

import * as vis from 'vis-network/dist/vis-network.esm.js';

// Required initialization for materialize dropdown:
document.addEventListener('DOMContentLoaded', function() {
    var elems = document.querySelectorAll('.dropdown-trigger');
    var instances = M.Dropdown.init(elems, {});
});

const presets = {
    "Empty": {
        program: "",
        rewriteRules: [],
    },
    "(* pa 2) → (<< pa 1)": {
        program: "(+ (* x 2) (* y 2))",
        rewriteRules: [
            ["(* pa 2)", "(<< pa 1)"],
        ],
    },
    "if-then-else": {
        program: "(and (if true (== (* 2 2) 4) false) (if false false (== (<< 2 1) 4)))",
        rewriteRules: [
            ["(and true true)", "true"],
            ["(if true pt pf)", "pt"],
            ["(if false pt pf)", "pf"],
            ["(* pa 2)", "(<< pa 1)"],
            ["(<< 2 1)", "4"],
            ["(== pa pa)", "true"],
        ],
    },
    "Pset #4": {
        program: "(land x y (f (g (f z))) (h y x) (h w x))",
        rewriteRules: [
            ["x", "y"],
            ["y", "(f z)"],
            ["(f (g (f z)))", "(h x y)"],
            ["(h y x)", "w"],
            ["(h w x)", "(f (g y))"],
        ],
    },
    "Congruence Closure w/ T/F + Inequalities": {
        program: "(land (eq (f x (g y)) (g (g (g y)))) (eq (f z z) x) (eq z (g y)) (not (eq (g (f x z)) (g (g (g z))))))",
        rewriteRules: [
            ["(f x (g y))", "(g (g (g y)))"],
            ["(f z z)", "x"],
            ["z", "(g y)"],

            ["(eq pa pa)", "true"],
            ["(not true)", "false"],
            ["(land true true true true)", "true"],
            ["(land false pa pb pc)", "false"],
            ["(land pa false pb pc)", "false"],
            ["(land pa pb false pc)", "false"],
            ["(land pa pb pc false)", "false"],
        ],
    },
};

import("../pkg/index.js").catch(console.error).then(wasm_module => {
    // For debugging purposes, we export the loaded WASM module into the global
    // window object:
    window.eggviz = wasm_module;

    const {
        LispylangEggvizRuntime
    } = wasm_module;

    // Create a new vis graph and set it to be rendered to a div:

    window.vis_nodes = new vis.DataSet([]);
    window.vis_edges = new vis.DataSet([]);

    let vis_graph = new vis.Network(
        document.getElementById("egraph"), {
            nodes: window.vis_nodes,
            edges: window.vis_edges
        },
        // Other configuration:
        {
            autoResize: false,
            interaction: {
                zoomView: false
            },
            physics: {
                solver: 'barnesHut',

                barnesHut: {
                    gravitationalConstant: -2000,
                    theta: 0.25,
                    centralGravity: 0.25,
                    springLength: 100,
                    springConstant: 0.04,
                    damping: 0.35,
                    avoidOverlap: 0,
                },
            },
        }
    );

    var in_graph = false;
    var runtime;

    function applyPreset(presetName) {
        const preset = presets[presetName];

        // Set the program accordingly:
        document.getElementById("program").value = preset.program;

        // Remove all defined rewrite rules and add rules from the preset:
        clear_rwr(null);

        for (let [src, dest] of preset.rewriteRules) {
            let row = document.createElement('div');
            row.className = "row";
            row.style = "margin-left: 1em";

            let src_p = document.createElement('p');
            src_p.style = "font-family: monospace; font-size: 10pt; margin-top: 1em; overflow: auto; white-space: nowrap";
            src_p.className = "col s5";
            src_p.textContent = src;
            row.appendChild(src_p)

            let arrow = document.createElement('p');
            arrow.textContent = "→";
            arrow.className = "col s1";
            arrow.style = "font-size: 20pt; margin-top: 0px";
            row.appendChild(arrow);

            let dest_p = document.createElement('p');
            dest_p.style = "font-family: monospace; font-size: 10pt; margin-top: 1em; overflow: auto; white-space: nowrap";
            dest_p.className = "col s5";
            dest_p.textContent = dest;
            row.appendChild(dest_p);

            let cfm = document.createElement('button');
            cfm.className = "red col";
            cfm.textContent = "⨯";
            cfm.style = "text-align: center; padding-left: 0.3em; padding-right: 0.3em; margin-top: 0.5em; margin-left: 0.5em";
            cfm.setAttribute("onclick", "confirm_rwr(event)");
            row.appendChild(cfm);

            let rwr = document.getElementById("rwr");
            rwr.appendChild(row);
        }

        check();
    }

    function initializePresets(presets) {
        const presetsUl = document.getElementById("preset-dropdown");
        while (presetsUl.firstChild) {
            presetsUl.removeChild(presetsUl.firstChild);
        }

        for (let [presetName, preset] of Object.entries(presets)) {
            let presetA = document.createElement('a');
            presetA.href = "#!";
            presetA.innerHTML = presetName;
            presetA.onclick = (() => applyPreset(presetName));

            let presetLi = document.createElement('li');
            presetLi.appendChild(presetA);

            presetsUl.appendChild(presetLi);
        }
    }

    initializePresets(presets);

    function colorWheel(id) {
        let rotation = id % 3;
        let offset = 128;
        var iteration = 0;
        for (var i = 0; i < id / 3; i++) {
            offset += 2 ** (6 - i);
        }
        let red = offset,
            blue = offset,
            green = offset;
        if (rotation === 1) {
            red = (red + 127) % 256;
        }
        if (rotation === 2) {
            green = (green + 127) % 256;
        }
        if (rotation === 3) {
            blue = (blue + 127) % 256;
        }
        return `rgb(${red}, ${green}, ${blue})`;
    }

    function check() {
        var success = true;
        let start = document.getElementById("start-reset");
        // Check if we have at least one rewrite rule
        let rwr = document.getElementById("rwr");
        if (rwr.children.length <= 1) {
            success = false;
        }
        if (success) {
            start.removeAttribute("disabled");
            if (!start.className.includes("green lighten-2")) {
                start.className += " green lighten-2";
            }
        } else {
            start.setAttribute("disabled", "true");
            start.className = start.className.replace(" green lighten-2", "");
        }
    }

    function makeRuntime(internal) {
        let program = document.getElementById("program").value;
        let rwr = document.getElementById("rwr");
        let rwrs = []
        for (var i = 1; i < rwr.children.length; i++) {
            let left = rwr.children[i].children[0].textContent;
            let right = rwr.children[i].children[2].textContent;
            rwrs.push(left, right);
        }
        try {
            runtime = LispylangEggvizRuntime.new(program, rwrs);
            window.runtime = runtime;
        } catch (ex) {
            if (internal) {
                throw "Internal error: parsing error should not occur in subsequent runtime generations."
            } else {
                start_reset();
                let footer = document.getElementById("footer");
                footer.style = "color: red";
                footer.textContent = ex;
            }
            return false;
        }
        redrawGraph();
        return true;
    }


    function firstGraph() {
        makeRuntime(true);
    }

    function nextGraph() {
        runtime.rewrite_auto();
        redrawGraph();
    }

    function prevGraph() {
        // TODO implement backtracking
        redrawGraph();
    }

    function clearGraph() {
        window.vis_nodes.clear();
        window.vis_edges.clear();
    }

    function redrawGraph() {
        let graph = runtime.current_graph();

        let processed_enode_ids = new Set();
        let processed_enode_edges = new Set();
        let processed_classes = new Set();
        let processed_eclass_edges = new Set();

        let class_nodes = new Map();

        for (let [eclass_id, enodes] of graph) {
            // Add square nodes for eclasses
            let vis_eclass = {
                id: eclass_id,
                label: "C" + eclass_id,
                shape: "circle",
                size: 50,
                font: "30px sans-serif black",
                group: Number(eclass_id),
            };
            window.vis_nodes.update(vis_eclass);
            processed_classes.add(vis_eclass.id);
            class_nodes[eclass_id] = vis_eclass.id;

            for (let [enode_id, enode] of enodes) {
                let enode_id_str = String(enode_id);
                let function_label = enode.get("label");
                let eclass_children = enode.get("children");

                let vis_enode = {
                    id: enode_id_str,
                    label: function_label,
                    shape: "box",
                    font: "20px sans-serif black",
                    margin: 15,
                    group: Number(eclass_id),
                    //color: colorWheel(Number(eclass_id)),
                };
                window.vis_nodes.update(vis_enode);
                processed_enode_ids.add(enode_id_str);

                // Add an edge from the eclass node to the enode.
                let vis_eclass_edge = {
                    id: eclass_id + ":" + enode_id_str,
                    from: eclass_id,
                    to: enode_id_str,
                    label: "",
                    width: 3,
                };
                window.vis_edges.update(vis_eclass_edge);
                processed_eclass_edges.add(vis_eclass_edge.id);

                for (let i = 0; i < eclass_children.length; i++) {
                    for (let [eclass_child_node_id, eclass_child_node] of graph.get(eclass_children[i])) {
                        let vis_edge = {
                            id: enode_id_str + ":" + eclass_children[i] + "$" + i,
                            from: enode_id_str,
                            to: eclass_children[i],
                            arrows: {
                                to: {
                                    enabled: true,
                                },
                            },
                            label: String(i),
                        };
                        window.vis_edges.update(vis_edge);
                        processed_enode_edges.add(vis_edge.id);
                    }
                }
            }
        }

        window.vis_nodes.forEach(node => {
            if (!processed_enode_ids.has(node.id) && !processed_classes.has(node.id)) {
                window.vis_nodes.remove(node.id);
            }
        });

        window.vis_edges.forEach(edge => {
            if (!processed_enode_edges.has(edge.id) && !processed_eclass_edges.has(edge.id)) {
                window.vis_edges.remove(edge.id);
            }
        });
    }

    window.check = check;
    window.firstGraph = firstGraph;
    window.nextGraph = nextGraph;

    function add_rwr(e) {
        let elt = document.createElement('div');
        elt.className = "row";
        elt.style = "margin-left: 1em";
        let src = document.createElement('input');
        src.style = "font-family: monospace; font-size: 10pt";
        src.className = "col s5";
        elt.appendChild(src);
        let arrow = document.createElement('p');
        arrow.textContent = "→";
        arrow.className = "col s1";
        arrow.style = "font-size: 20pt; margin-top: 0px";
        elt.appendChild(arrow);
        let dest = document.createElement('input');
        dest.style = "font-family: monospace; font-size: 10pt";
        dest.className = "col s5";
        elt.appendChild(dest);
        let cfm = document.createElement('button');
        cfm.className = "green col";
        cfm.textContent = "✓";
        cfm.style = "text-align: center; padding-left: 0.3em; padding-right: 0.3em; margin-top: 0.5em; margin-left: 0.5em";
        cfm.setAttribute("onclick", "confirm_rwr(event)");
        elt.appendChild(cfm);
        let rwr = document.getElementById("rwr");
        rwr.appendChild(elt);
    }

    window.add_rwr = add_rwr;

    function clear_rwr(e) {
        let rwr = document.getElementById("rwr");
        let ch = rwr.children;
        let len = ch.length;
        for (var i = len - 1; i >= 1; i--) {
            ch[i].remove();
        }
        check();
    }

    window.clear_rwr = clear_rwr;

    function confirm_rwr(e) {
        let start = document.getElementById("start-reset");
        let row = e.target.parentElement;
        let cfm = row.children[3];
        if (cfm.textContent === "⨯") {
            // Deleting a confirmed rerwite rule
            row.remove();
        } else {
            let src = row.children[0].value;
            let dest = row.children[2].value;
            let src_p = document.createElement('p');
            src_p.style = "font-family: monospace; font-size: 10pt; margin-top: 1em; overflow: auto; white-space: nowrap";
            src_p.className = "col s5";
            src_p.textContent = src;
            let dest_p = document.createElement('p');
            dest_p.style = "font-family: monospace; font-size: 10pt; margin-top: 1em; overflow: auto; white-space: nowrap";
            dest_p.className = "col s5";
            dest_p.textContent = dest;
            let arrow = row.children[1];
            cfm.className = "red col";
            cfm.textContent = "⨯";
            row.replaceChildren(src_p, arrow, dest_p, cfm);
        }
        check();
    }

    window.confirm_rwr = confirm_rwr;

    function start_reset() {
        in_graph = !in_graph;
        if (in_graph) {
            // Start graphing!
            let btn_preset = document.getElementById("btn-preset");
            btn_preset.setAttribute("disabled", "true");

            let btn_add_rwr = document.getElementById("add-rwr");
            btn_add_rwr.textContent = "← Previous"
            btn_add_rwr.className = btn_add_rwr.className.replace("cyan", "green");
            btn_add_rwr.onclick = prevGraph;
            let btn_clear_rwr = document.getElementById("clear-rwr");
            btn_clear_rwr.textContent = "Auto →";
            btn_clear_rwr.onclick = nextGraph;
            btn_clear_rwr.className = btn_add_rwr.className.replace("cyan", "green");
            document.getElementById("program").setAttribute("disabled", "true");
            let rwr = document.getElementById('rwr');
            if (rwr.children.length <= 1) {
                start_reset();
                return;
            }
            for (var i = 1; i < rwr.children.length; i++) {
                rwr.children[i].style.cursor = "pointer";
                rwr.children[i].children[3].setAttribute("disabled", "true");
                let cl = rwr.children[i].children[3].className;
                rwr.children[i].children[3].className = cl.replace("red", "");
            }
            let start = document.getElementById("start-reset");
            start.textContent = "Reset ↺";
            start.className = start.className.replace("green", "red");
            for (let i = 1; i < rwr.children.length; i++) {
                rwr.children[i].onmouseenter = function(e) {
                    e.target.className += " green lighten-2";
                };
                rwr.children[i].onmouseleave = function(e) {
                    e.target.className = e.target.className.replace(" green lighten-2", "");
                };
                rwr.children[i].onclick = (e) => {
                    let rule_name = "rwr#" + (i - 1);
                    runtime.rewrite_rule(rule_name);
                    redrawGraph();
                };
            }
            if (!makeRuntime(false)) {
                return;
            }
            let footer = document.getElementById("footer");
            footer.style = "color: black";
            footer.textContent = "Click on a rewrite rule to apply it, or click on the Auto button to apply all rewrite rules once. Click the back arrow to go back a step.";
        } else {
            // Remove graph and re-enable program/rwr panes
            clearGraph();

            let btn_preset = document.getElementById("btn-preset");
            btn_preset.removeAttribute("disabled");

            let btn_add_rwr = document.getElementById("add-rwr");
            btn_add_rwr.textContent = "Add"
            btn_add_rwr.className = btn_add_rwr.className.replace("green", "cyan");
            btn_add_rwr.onclick = function(e) {
                add_rwr(e)
            };
            let btn_clear_rwr = document.getElementById("clear-rwr");
            btn_clear_rwr.textContent = "Clear";
            btn_clear_rwr.className = btn_add_rwr.className.replace("green", "cyan");
            btn_clear_rwr.onclick = function(e) {
                clear_rwr(e)
            };
            document.getElementById("program").removeAttribute("disabled");
            let rwr = document.getElementById('rwr');
            for (var i = 1; i < rwr.children.length; i++) {
                rwr.children[i].onclick = function(e) {};
                rwr.children[i].onmouseenter = function(e) {};
                rwr.children[i].onmouseleave = function(e) {};
                rwr.children[i].children[3].removeAttribute("disabled");
                rwr.children[i].children[3].className += " red";
            }
            let start = document.getElementById("start-reset");
            start.textContent = "Graph!";
            start.className = start.className.replace("red", "green");
            footer.textContent = "Write your program in the top bar, add some rewrite rules on the left pane, then press the graph button!";
        }
    }

    window.start_reset = start_reset;

    function modify_program(e) {}

    window.modify_program = modify_program;

});