import init, { WasmExport } from './pkg/org_wasm.js';


// load the wasm module
async function run() {
  await init();
}
await run();
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


// handle the dropdown selector
let affiliated_string = await (await fetch("./files/affiliated.org")).text();
let default_string = await (await fetch("./files/default.org")).text();
let footnotes_string = await (await fetch("./files/footnotes.org")).text();

// key items in the file
let textbox = document.getElementById("textbox");
let views = document.querySelectorAll(".tabcontent");
let switch_buttons = document.querySelectorAll(".tablinks");
let display_select = document.getElementById("display-select");

// set it like so the textbox maintains the previous selection
// the selectbox doesn't reset to "default" on refresh
select_func(display_select.value);

function select_func(val) {
  switch (val) {
    case "affiliated": {
      textbox.value = affiliated_string;
      break;
    }
    case "default": {
      textbox.value = default_string;
      break;
    }
    case "footnotes": {
      textbox.value = footnotes_string;
      break;
    }
  }
}

display_select.addEventListener('change', async (e) => {
  select_func(e.target.value);
  reparse();
});


let currElem = view_dict["rendered"];

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
  let result = parse_func((textbox.value).concat("\n"));
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
let throttled_reparse = throttle(reparse, 35);

textbox.addEventListener('input', throttled_reparse);

toggleView("rendered", "rendered-parse");
reparse();

