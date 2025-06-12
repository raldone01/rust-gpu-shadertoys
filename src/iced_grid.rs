use iced_core::Size;

// Compute optimal grid layout (rows, cols) for cell count while attempting to keep the aspect ratio close to the provided aspect ratio.
#[must_use]
fn optimal_grid(cell_count: usize, aspect: Size) -> Size<usize> {
  // Handle edge cases for 0 or 1 cells.
  if cell_count == 0 {
    return Size::new(0, 0);
  }
  if cell_count == 1 {
    return Size::new(1, 1);
  }

  // The target aspect ratio (width / height). Add a small epsilon to avoid division by zero.
  let target_aspect = aspect.width / (aspect.height + f32::EPSILON);

  let mut best_layout = Size::new(1, cell_count);
  let mut min_aspect_diff = f32::INFINITY;

  // Iterate through all possible row counts from 1 to cell_count.
  // This is a simple and robust way to find the global optimum.
  for rows in 1..=cell_count {
    // Calculate the number of columns needed to fit all cells for the current row count.
    // This is equivalent to `ceil(cell_count / rows)`.
    let cols = cell_count.div_ceil(rows);

    // The aspect ratio of the current grid layout.
    let grid_aspect = cols as f32 / rows as f32;

    // Calculate the difference from the target aspect ratio.
    let diff = (grid_aspect - target_aspect).abs();

    // If this layout is better than the best one we've found so far, update it.
    if diff < min_aspect_diff {
      min_aspect_diff = diff;
      best_layout = Size::new(rows, cols);
    }
  }

  best_layout
}
