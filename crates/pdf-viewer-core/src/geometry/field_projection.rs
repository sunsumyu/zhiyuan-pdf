use crate::models::{FieldProjection, FieldProjectionRequest, RectBox};

pub fn resolve_field_projection(request: &FieldProjectionRequest) -> FieldProjection {
    let editable_left = if request.has_field_meta {
        request.value_left
    } else {
        request.group_left
    };
    let full_group_width = (request.group_right - request.group_left + 4.0).max(18.0);
    let shell_width =
        ((request.group_right.max(request.slot_right) - request.group_left) + 8.0).max(56.0);
    let label_width = ((request.label_right - request.label_left) + 4.0).max(12.0);
    let value_width = ((request.value_right.max(editable_left) - editable_left) + 8.0).max(24.0);
    let editor_width = ((request.slot_right - editable_left) + 12.0).max(56.0);
    let height = ((request.top - request.bottom) + 4.0).max(18.0);
    let top = (request.page_height - request.top) - 2.0;
    let text_left = request.group_left - 2.0;
    let shell_left = request.group_left - 4.0;
    let label_left = request.label_left - 2.0;
    let value_left = editable_left - 2.0;
    let editor_left = editable_left - 2.0;

    FieldProjection {
        text_box: RectBox {
            left: text_left,
            top,
            width: full_group_width,
            height,
        },
        shell_box: RectBox {
            left: shell_left,
            top,
            width: shell_width,
            height,
        },
        label_box: RectBox {
            left: label_left,
            top,
            width: label_width,
            height,
        },
        value_box: RectBox {
            left: value_left,
            top,
            width: value_width,
            height,
        },
        editor_box: RectBox {
            left: editor_left,
            top,
            width: editor_width,
            height,
        },
    }
}
