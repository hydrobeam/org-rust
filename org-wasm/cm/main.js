import { Compartment, EditorState } from "@codemirror/state"
import { EditorView, keymap } from "@codemirror/view"
import { indentWithTab } from "@codemirror/commands"
import { vim } from "@replit/codemirror-vim"
import { basicSetup } from "codemirror";


// import { WasmExport } from '../pkg/';

// import { wasm }  from "../pkg"
// let exporter;
// import("../pkg").then((wasm) => {
//   console.log(wasm);
//   exporter = new wasm.WasmExport();
//   // js.greet("WebAssembly with npm");
// });

import init, { WasmExport } from "./pkg/org_wasm.js"
import wasm from "./pkg/org_wasm_bg.wasm"

await init(await wasm());
let exporter = new WasmExport();

let
  parse_dict = {
    "org": exporter.to_org.bind(exporter),
    "html": exporter.to_html.bind(exporter),
  };

// the function that's used to parse the input
let parse_func = parse_dict["html"];

// dict of tab views
let view_dict = {
  "org": document.getElementById("org"),
  "raw": document.getElementById("raw"),
  "rendered": document.getElementById("rendered"),
}

// import affiliated_string from "./files/affiliated.org";
// import default_string from "./files/default.org";
// import footnotes_string from "./files/footnotes.org";

// handle the dropdown selector
let affiliated_string = await (await fetch("./files/affiliated.org")).text();
let default_string = await (await fetch("./files/default.org")).text();
let footnotes_string = await (await fetch("./files/footnotes.org")).text();
let currElem = view_dict["rendered"];

// key items in the file
let textbox = document.getElementById("textbox");
let views = document.querySelectorAll(".tabcontent");
let switch_buttons = document.querySelectorAll(".tablinks");
let display_select = document.getElementById("display-select");

let vim_button = document.getElementById("vim");


let vim_keymap = new Compartment();

let throttled_reparse = throttle(reparse, 35);

let editor = new EditorView({
  parent: textbox,
  state: EditorState.create({
    extensions: [
      vim_keymap.of([]),
      keymap.of(indentWithTab),
      basicSetup,
      EditorView.updateListener.of(function (e) {
        if (e.docChanged) {
          throttled_reparse()
        }
      }),

    ]
  })
})

vim_button.addEventListener("click", () => {
  let on = vim_button.checked;
  editor.dispatch({
    effects: vim_keymap.reconfigure(on ? vim() : [])
  })
})

// set it like so the textbox maintains the previous selection
// the selectbox doesn't reset to "default" on refresh
select_func(display_select.value);

function select_func(val) {
  switch (val) {
    case "affiliated": {
      editor.dispatch({ changes: { from: 0, to: editor.state.doc.length, insert: affiliated_string } });
      break;
    }
    case "default": {
      editor.dispatch({ changes: { from: 0, to: editor.state.doc.length, insert: default_string } });
      break;
    }
    case "footnotes": {
      editor.dispatch({ changes: { from: 0, to: editor.state.doc.length, insert: footnotes_string } });
      break;
    }
  }
}

display_select.addEventListener('change', async (e) => {
  select_func(e.target.value);
  reparse();
});



switch_buttons.forEach((elem) => {
  elem.addEventListener('click', () => {
    switch (elem.id) {
      case "org-parse":
        parse_func = parse_dict["org"];
        toggleView("org", elem.id);
        currElem = view_dict["org"];
        break;
      case "rendered-parse":
        parse_func = parse_dict["html"];
        toggleView("rendered", elem.id);
        currElem = view_dict["rendered"];
        break;
      case "raw-parse":
        parse_func = parse_dict["html"];
        toggleView("raw", elem.id);
        currElem = view_dict["raw"];
        break;
    }
    reparse();

  })
})

// Handles tab switching
function toggleView(name, button_name) {
  views.forEach((elem) => {
    if (elem.id === name) {
      elem.style.display = "block";
    } else {
      elem.style.display = 'none';
    }
  })

  switch_buttons.forEach((elem) => {
    if (elem.id === button_name) {
      elem.classList.remove("inactive");
      elem.classList.add("active");
    } else {
      elem.classList.remove("active");
      elem.classList.add("inactive");
    }
  })
}

function reparse() {
  let result = parse_func(editor.state.doc.toString().concat("\n"));
  // actually changing srcdoc causes extreme white flashing.
  // updating the iframe like this is much better
  if (currElem === view_dict["rendered"]) {
    currElem.contentDocument.body.innerHTML = result;
  } else {
    currElem.textContent = result;
  }
}

// prevent excessive reparsing on repeated inputs
function throttle(cb, delay) {
  let shouldWait = false
  let waitingArgs
  const timeoutFunc = () => {
    if (waitingArgs == null) {
      shouldWait = false
    } else {
      cb(...waitingArgs)
      waitingArgs = null
      setTimeout(timeoutFunc, delay)
    }
  }

  return (...args) => {
    if (shouldWait) {
      waitingArgs = args
      return
    }

    cb(...args)
    shouldWait = true
    setTimeout(timeoutFunc, delay)
  }
}

toggleView("rendered", "rendered-parse");
reparse();

