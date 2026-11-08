//! #12196: a live Gradient drag (`gradient.fill.create`) masks the new fill layer with the
//! selection and drops the selection in the same History step, while `new_layer` (used by the
//! canvas preview during the drag) masks the preview without mutating the document's selection.

use photocraft_geom::Rect;
use serde_json::json;

use super::{CREATE, LayerId, new_layer};
use crate::Session;

fn session(depth: u32) -> Session {
    let mut s = Session::new();
    s.execute("file.new", json!({"width": 800, "height": 600, "depth": depth, "background": "white"})).unwrap();
    s
}

#[test]
fn create_masks_with_the_selection_drops_it_and_undo_restores_it() {
    let bounds = Rect::new(200, 200, 400, 200);
    let drag = json!({"from": [200, 200], "to": [600, 400]});
    for depth in [8, 16, 32] {
        let mut s = session(depth);
        s.execute("select.rect", json!({"x": 200, "y": 200, "width": 400, "height": 200})).unwrap();

        // The live drag preview calls `new_layer`: it is masked by the selection and leaves
        // the document's selection intact while dragging.
        let preview = new_layer(&s, &s.active().unwrap().doc, &drag).unwrap();
        let preview_mask = preview.mask.as_ref().expect("preview is masked by the selection");
        assert_eq!(preview_mask.surface.content_bounds(), bounds, "{depth}-bit preview mask bounds");
        assert!(s.active().unwrap().doc.selection.is_some(), "{depth}-bit preview keeps selection");

        let r = s.execute(CREATE, drag.clone()).unwrap();
        let id = LayerId(r["layer"].as_u64().unwrap());
        let d = s.active().unwrap();
        let l = d.doc.layer(id).unwrap();
        assert_eq!(l.name, "Gradient Fill 1", "{depth}-bit");
        let mask = l.mask.as_ref().expect("selection becomes the fill layer's mask");
        assert_eq!(mask.surface.content_bounds(), bounds, "{depth}-bit mask bounds");
        assert_eq!(mask.value(300, 300), 1.0, "{depth}-bit inside selection");
        assert_eq!(mask.value(100, 100), 0.0, "{depth}-bit outside selection");
        assert!(d.doc.selection.is_none(), "{depth}-bit selection is dropped");

        // One undo removes the layer and brings the selection back.
        s.execute("edit.undo", json!({})).unwrap();
        let d = s.active().unwrap();
        assert!(d.doc.layer(id).is_none(), "{depth}-bit undo removes the layer");
        let sel = d.doc.selection.as_ref().expect("undo brings the selection back");
        assert_eq!(sel.content_bounds(), bounds, "{depth}-bit restored selection bounds");
    }
}

#[test]
fn create_without_a_selection_adds_an_unmasked_fill_layer() {
    for depth in [8, 16, 32] {
        let mut s = session(depth);
        let r = s.execute(CREATE, json!({"from": [100, 100], "to": [700, 500]})).unwrap();
        let id = LayerId(r["layer"].as_u64().unwrap());
        let d = s.active().unwrap();
        let l = d.doc.layer(id).unwrap();
        assert!(l.mask.is_none(), "{depth}-bit without selection has no mask");
        assert!(d.doc.selection.is_none(), "{depth}-bit still has no selection");
    }
}
