ObjC.import('CoreGraphics');
ObjC.import('Foundation');
function run(argv) {
  const owner = argv[0] || 'keyprompter';
  const list = ObjC.deepUnwrap(ObjC.castRefToObject($.CGWindowListCopyWindowInfo($.kCGWindowListOptionOnScreenOnly, 0)));
  const mine = list.filter(w => w.kCGWindowOwnerName === owner)
    .map(w => ({ layer: w.kCGWindowLayer, bounds: w.kCGWindowBounds, alpha: w.kCGWindowAlpha }));
  return JSON.stringify(mine);
}
