/* preact/jsx-runtime over the inlined runtime, so vendored packages that were compiled with the
   automatic JSX runtime share the page's one Preact. */
(function (p) {
  function jsx(type, props, key) {
    var o = {};
    for (var k in props) o[k] = props[k];
    if (key !== undefined) o.key = key;
    return p.h(type, o);
  }
  window.preactJsx = { jsx: jsx, jsxs: jsx, jsxDEV: jsx, Fragment: p.Fragment };
})(window.preact);
