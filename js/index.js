import 'materialize-css/dist/css/materialize.min.css';
import 'materialize-css/dist/js/materialize.min';

import("../pkg/index.js").catch(console.error).then(wasm_module => {
    // For debugging purposes, we export the loaded WASM module into the global
    // window object:
    window.eggviz = wasm_module;

    const {
        LispylangEggvizRuntime
    } = wasm_module;

    var in_graph = false;

    function check() {
        var success = true; // TODO
        let start = document.getElementById("start-reset");
        // Check if we have at least one rewrite rule
        let rwr = document.getElementById("rwr");
        if (rwr.children.length <= 1) {
            success = false;
        }
        if (success) {
            start.removeAttribute("disabled");
        } else {
            start.setAttribute("disabled", "true");
        }
    }

    window.check = check;

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
            document.getElementById("add-rwr").setAttribute("disabled", "true");
            document.getElementById("clear-rwr").setAttribute("disabled", "true");
            document.getElementById("program").setAttribute("disabled", "true");
            let rwr = document.getElementById('rwr');
            for (var i = 1; i < rwr.children.length; i++) {
                rwr.children[i].children[3].setAttribute("disabled", "true");
                let cl = rwr.children[i].children[3].className;
                rwr.children[i].children[3].className = cl.replace("red", "");
            }
            let start = document.getElementById("start-reset");
            start.textContent = "Reset ↺";
            start.className = start.className.replace("green", "red");

            let program = document.getElementById("program").value;
            var rwrs = [];
            for (var i = 1; i < rwr.children.length; i++) {
                let left = rwr.children[i].children[0].textContent;
                let right = rwr.children[i].children[2].textContent;
                rwrs.push(left, right);
            }
            let footer = document.getElementById("footer");

            // Time for magic!
            var runtime;
            try {
                runtime = LispylangEggvizRuntime.new(program, rwrs);
            } catch (ex) {
                let footer = document.getElementById("footer");
                footer.style = "color: red";
                footer.textContent = "Parsing error: " + ex;
                start_reset();
                return;
            }
            footer.style = "color: black";
            footer.textContent = "Write your program in the top bar, add some rewrite rules on the left pane, then press the graph button!";
        } else {
            // Remove graph and re-enable program/rwr panes
            document.getElementById("add-rwr").removeAttribute("disabled");
            document.getElementById("clear-rwr").removeAttribute("disabled");
            document.getElementById("program").removeAttribute("disabled");
            let rwr = document.getElementById('rwr');
            for (var i = 1; i < rwr.children.length; i++) {
                rwr.children[i].children[3].removeAttribute("disabled");
                rwr.children[i].children[3].className += " red";
            }
            let start = document.getElementById("start-reset");
            start.textContent = "Graph!";
            start.className = start.className.replace("red", "green");
        }
    }

    window.start_reset = start_reset;

    function modify_program(e) {
        console.log("Modified program!");
    }

    window.modify_program = modify_program;

});