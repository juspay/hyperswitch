Cypress.Screenshot.defaults({
  blackout: [".secret-info", "[data-hide=true]"],
  capture: "runner",
  overwrite: false,

  onBeforeScreenshot($el) {
    const $clock = $el.find(".clock");

    if ($clock) {
      $clock.hide();
    }
  },

  onAfterScreenshot($el, props) {
    const $clock = $el.find(".clock");

    if ($clock) {
      $clock.show();
    }
  },
});
