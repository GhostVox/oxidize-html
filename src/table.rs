use crate::{Display, SizeValue, StyledNode};

pub fn normalize_tables(root: &mut StyledNode, available_width: f32) {
    visit(root, available_width);
}

fn visit(node: &mut StyledNode, parent_width: f32) {
    if node.tag.as_deref() == Some("table") {
        normalize_table(node, parent_width);
    }

    let width = node_width(&node.style.width, parent_width).unwrap_or(parent_width);
    for child in &mut node.children {
        visit(child, width);
    }
}

fn normalize_table(table: &mut StyledNode, available_width: f32) {
    table.style.display = Display::Table;
    let table_width = node_width(&table.style.width, available_width).unwrap_or(available_width);
    if matches!(table.style.width, SizeValue::Auto) {
        table.style.width = SizeValue::Px(table_width);
    }

    let mut rows = Vec::new();
    collect_rows(table, &mut rows);

    if rows.is_empty() {
        return;
    }

    let col_count = rows
        .iter()
        .map(|r| {
            r.children
                .iter()
                .filter(|n| matches!(n.tag.as_deref(), Some("td" | "th")))
                .count()
        })
        .max()
        .unwrap_or(0);
    if col_count == 0 {
        return;
    }

    let mut widths: Vec<Option<f32>> = vec![None; col_count];
    for row in rows {
        let mut col = 0;
        for cell in row
            .children
            .iter()
            .filter(|n| matches!(n.tag.as_deref(), Some("td" | "th")))
        {
            let span = colspan(cell);
            if let Some(width) = node_width(&cell.style.width, table_width) {
                let each = width / span as f32;
                for i in col..(col + span).min(col_count) {
                    widths[i] = Some(widths[i].unwrap_or(each).max(each));
                }
            }
            col += span;
        }
    }

    let explicit_total: f32 = widths.iter().flatten().sum();
    let remaining_cols = widths.iter().filter(|w| w.is_none()).count();
    let leftover = (table_width - explicit_total).max(0.0);
    let fallback = if remaining_cols > 0 {
        leftover / remaining_cols as f32
    } else {
        table_width / col_count as f32
    };
    let resolved: Vec<f32> = widths.into_iter().map(|w| w.unwrap_or(fallback)).collect();

    for_each_row_mut(table, &mut |row| {
        let mut col = 0;
        for cell in row
            .children
            .iter_mut()
            .filter(|n| matches!(n.tag.as_deref(), Some("td" | "th")))
        {
            cell.style.display = Display::TableCell;
            let span = colspan(cell).max(1);
            let end = (col + span).min(resolved.len());
            let width = resolved[col..end].iter().sum();
            cell.style.width = SizeValue::Px(width);
            col = end;
        }
    });
}

fn colspan(cell: &StyledNode) -> usize {
    cell.attrs
        .get("colspan")
        .and_then(|v| v.parse::<usize>().ok())
        .filter(|v| *v > 0)
        .unwrap_or(1)
}

fn node_width(size: &SizeValue, parent_width: f32) -> Option<f32> {
    match size {
        SizeValue::Px(px) => Some(*px),
        SizeValue::Percent(pct) => Some((pct / 100.0) * parent_width),
        SizeValue::Auto => None,
    }
}

fn collect_rows<'a>(node: &'a StyledNode, rows: &mut Vec<&'a StyledNode>) {
    if node.tag.as_deref() == Some("tr") {
        rows.push(node);
        return;
    }
    for child in &node.children {
        if child.tag.as_deref() == Some("table") {
            continue;
        }
        collect_rows(child, rows);
    }
}

fn for_each_row_mut(node: &mut StyledNode, f: &mut impl FnMut(&mut StyledNode)) {
    if node.tag.as_deref() == Some("tr") {
        f(node);
        return;
    }
    for child in &mut node.children {
        if child.tag.as_deref() == Some("table") {
            continue;
        }
        for_each_row_mut(child, f);
    }
}

#[cfg(test)]
mod tests {
    use crate::{HtmlRenderer, SizeValue, StyledNode};

    fn find_first_tag<'a>(node: &'a StyledNode, tag: &str) -> Option<&'a StyledNode> {
        if node.tag.as_deref() == Some(tag) {
            return Some(node);
        }
        for child in &node.children {
            if let Some(found) = find_first_tag(child, tag) {
                return Some(found);
            }
        }
        None
    }

    #[test]
    fn distributes_table_width_to_missing_columns() {
        let html =
            r#"<table width="600"><tr><td width="200">A</td><td>B</td><td>C</td></tr></table>"#;
        let mut renderer = HtmlRenderer::default();
        let mut tree = renderer.style_tree(html);
        super::normalize_tables(&mut tree, 600.0);
        let row = find_first_tag(&tree, "tr").expect("row");
        let widths: Vec<f32> = row
            .children
            .iter()
            .map(|c| match c.style.width {
                SizeValue::Px(px) => px,
                _ => 0.0,
            })
            .collect();
        assert_eq!(widths, vec![200.0, 200.0, 200.0]);
    }

    #[test]
    fn nested_table_columns_not_corrupted() {
        let html = r#"
            <table width="600">
              <tr>
                <td width="300">Left</td>
                <td width="300">
                  <table width="200">
                    <tr><td width="100">A</td><td width="100">B</td></tr>
                  </table>
                </td>
              </tr>
            </table>
        "#;

        let mut renderer = HtmlRenderer::default();
        let mut tree = renderer.style_tree(html);
        super::normalize_tables(&mut tree, 600.0);

        fn find_tables<'a>(node: &'a StyledNode, out: &mut Vec<&'a StyledNode>) {
            if node.tag.as_deref() == Some("table") {
                out.push(node);
            }
            for child in &node.children {
                find_tables(child, out);
            }
        }
        let mut tables = Vec::new();
        find_tables(&tree, &mut tables);
        assert!(tables.len() >= 2);
        let inner = tables[1];
        let inner_row = find_first_tag(inner, "tr").expect("inner row");
        let widths: Vec<f32> = inner_row
            .children
            .iter()
            .filter(|n| matches!(n.tag.as_deref(), Some("td" | "th")))
            .map(|c| match c.style.width {
                SizeValue::Px(px) => px,
                _ => 0.0,
            })
            .collect();
        assert_eq!(widths, vec![100.0, 100.0]);
    }
}
