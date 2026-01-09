if (window.poorMansEntryPointMissed) {
  main();
} else {
  window.poorMansEntryPoint = main;
}

function main() {
  console.log(window.wasmBindings.mdr());
}
