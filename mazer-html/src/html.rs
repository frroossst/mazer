/// A composable way to build HTML documents.
/// ```no_run
///     use mazer_html::html;
/// 
///     let html_repr = div().img(Some("image.png")).child(
///        p().text("Hello, world!")
///     );
/// 
///     let html_output: HTML = html_repr.into();
/// ```










