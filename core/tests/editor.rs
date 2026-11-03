use topograph_core::{
    algebra::{Form, Primitive},
    editor::Editor,
};
#[test]
fn face_edit_is_transactional_and_preserves_other_fixed_values() {
    let mut e = Editor::default();
    let a = e.fixed[0].face.clone();
    e.edit_face(&a, 5.into()).unwrap();
    assert_eq!(e.form, Form::small(5, -4, 1));
    assert_eq!(e.fixed[1].value, 1.into());
    assert_eq!(e.fixed[2].value, 2.into());
    e.replace(2, Primitive::small(1, 2)).unwrap();
    let old = e.form.clone();
    assert!(e.apply_labels(&[0.into(), 0.into(), 1.into()]).is_err());
    assert_eq!(e.form, old);
    e.fixed.pop();
    assert!(e.edit_face(&a, 3.into()).is_err());
    assert_eq!(e.form, old);
    assert!(e.edit_face(&Primitive::small(7, 3), 3.into()).is_err());
}
