ObjC.import('CoreGraphics');
ObjC.import('Foundation');
// windows.js [owner] [pid]: on-screen windows of that app as JSON, or with a pid, the id of its first window.
function run(argv) {
  const owner = argv[0] || 'keyprompter';
  const list = ObjC.deepUnwrap(ObjC.castRefToObject($.CGWindowListCopyWindowInfo($.kCGWindowListOptionOnScreenOnly, 0)));
  const mine = list.filter(w => w.kCGWindowOwnerName === owner)
    .map(w => ({ id: w.kCGWindowNumber, pid: w.kCGWindowOwnerPID, layer: w.kCGWindowLayer, bounds: w.kCGWindowBounds, alpha: w.kCGWindowAlpha }));
  if (argv[1]) {
    const w = mine.find(w => w.pid === Number(argv[1]));
    if (!w) throw new Error('no on-screen window for pid ' + argv[1]);
    return String(w.id);
  }
  return JSON.stringify(mine);
}
