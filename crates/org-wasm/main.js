import { Compartment, EditorState } from "@codemirror/state"
import { EditorView, keymap } from "@codemirror/view"
import { indentWithTab } from "@codemirror/commands"
import { vim } from "@replit/codemirror-vim"
import { basicSetup } from "codemirror";
import { org } from "./editor.ts";
import { WasmExport } from "./pkg/org_wasm.js"

let exporter = new WasmExport();

let
  parse_dict = {
    "org": exporter.to_org.bind(exporter),
    "html": exporter.to_html.bind(exporter),
  };

// the function that's used to parse the input
let parse_func = parse_dict["html"];

// dict of tab views
const view_dict = {
  "org": document.getElementById("org"),
  "raw": document.getElementById("raw"),
  "rendered": document.getElementById("rendered"),
}

// handle the dropdown selector
// webpack inlines these in the generated js file
import affiliated_string from "./static/affiliated.org";
import default_string from "./static/default.org";
import footnotes_string from "./static/footnotes.org";
import images_string from "./static/images.org";

let currElem = view_dict["rendered"];

// key items in the file
const textbox = document.getElementById("textbox");
const views = document.querySelectorAll(".tabcontent");
const switch_buttons = document.querySelectorAll(".tablinks");
const contentButtons = document.querySelectorAll(".contentlinks");

const vim_button = document.getElementById("vim");

const vim_keymap = new Compartment();

const throttled_reparse = throttle(reparse, 35);

const gutterColour = getComputedStyle(document.body)
  .getPropertyValue('--gutter-colour');

const lineHover = getComputedStyle(document.body)
  .getPropertyValue('--line-hover');

const selection = getComputedStyle(document.body)
  .getPropertyValue('--line-selection');

// kinda greeny
const theme = EditorView.theme(
  {
    // don't know what this implies, just stolen from the @codemirror/theme-one-dark
    "&.cm-focused > .cm-scroller > .cm-selectionLayer .cm-selectionBackground, .cm-selectionBackground, .cm-content ::selection": { backgroundColor: selection },

    // these are self explanatory
    '.cm-activeLine': {
      backgroundColor: lineHover,
    },
    '.cm-gutters': {
      backgroundColor: gutterColour,
    },
    '.cm-activeLineGutter': {
      backgroundColor: lineHover,
    },
  });

const editor = new EditorView({
  parent: textbox,
  state: EditorState.create({
    extensions: [
      theme,
      EditorView.lineWrapping,
      vim_keymap.of([]),
      org,
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
  const on = vim_button.checked;
  editor.dispatch({
    effects: vim_keymap.reconfigure(on ? vim() : [])
  })
})

// FIXME: local document storage to not have to worry about this
// obselete: set it like so the textbox maintains the previous selection
// the selectbox doesn't reset to "default" on refresh
select_func("default");

// scroll to the top of the editor, it ends up in the middle for some reason without
// this
editor.scrollDOM.scroll(0, 1)


function select_func(val) {
  switch (val) {
    case "affiliated": {
      editor.dispatch({
        changes: { from: 0, to: editor.state.doc.length, insert: affiliated_string },
      });
      break;
    }
    case "default": {
      editor.dispatch({
        changes: { from: 0, to: editor.state.doc.length, insert: default_string },
      });
      break;
    }
    case "footnotes": {
      editor.dispatch({
        changes: { from: 0, to: editor.state.doc.length, insert: footnotes_string },
      });
      break;
    }
    case "images": {
      editor.dispatch({
        changes: { from: 0, to: editor.state.doc.length, insert: images_string },
      });
      break;
    }
  }
}


contentButtons.forEach((elem) => {
  elem.addEventListener('click', () => {
    select_func(elem.value);
    reparse();
  })
})


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
  const result = parse_func(editor.state.doc.toString().concat("\n"));
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
