pub struct CliPrinterConfig {
    pub summary_total_width: usize,
    pub list_title_width: usize,
    pub list_tags_width: usize,
    pub list_type_width: usize,
    pub search_title_width: usize,
    pub search_type_width: usize,
}

impl Default for CliPrinterConfig {
    fn default() -> Self {
        Self {
            summary_total_width: 42,
            list_title_width: 50,
            list_tags_width: 50,
            list_type_width: 10,
            search_title_width: 50,
            search_type_width: 10,
        }
    }
}

pub struct CliPrinter {
    config: CliPrinterConfig,
}

impl CliPrinter {
    pub fn new(config: CliPrinterConfig) -> Self {
        Self { config }
    }

    #[allow(clippy::should_implement_trait)]
    pub fn with_default_config() -> Self {
        Self::new(CliPrinterConfig::default())
    }

    #[allow(clippy::too_many_arguments)]
    pub fn print_document_summary(
        &self,
        doc_type: &str,
        title: &str,
        authors: Option<&Vec<String>>,
        year: Option<i32>,
        source: Option<&str>,
        url: Option<&str>,
        tags: &[String],
        notes: Option<&str>,
    ) {
        let total_width = self.config.summary_total_width;
        let content_width = total_width - 12;

        println!();
        println!("┌{:─<width$}┐", "", width = total_width);
        println!("│  Add Document {:width$}│", "", width = total_width - 15);
        println!("├{:─<width$}┤", "", width = total_width);

        let w = content_width;
        println!("│  Type:    {:width$} │", doc_type, width = w);
        println!("│  Title:   {:width$} │", truncate(title, w), width = w);

        if let Some(authors_list) = authors {
            println!(
                "│  Authors: {:width$} │",
                truncate(&authors_list.join(", "), w),
                width = w
            );
        }

        if let Some(y) = year {
            println!("│  Year:    {:width$} │", y, width = w);
        } else {
            println!("│  Year:    {:width$} │", "-", width = w);
        }

        if let Some(s) = source {
            println!("│  Source:  {:width$} │", truncate(s, w), width = w);
        } else {
            println!("│  Source:  {:width$} │", "-", width = w);
        }

        if let Some(u) = url {
            println!("│  URL:     {:width$} │", truncate(u, w), width = w);
        } else {
            println!("│  URL:     {:width$} │", "-", width = w);
        }

        if tags.is_empty() {
            println!("│  Tags:    {:width$} │", "-", width = w);
        } else {
            println!(
                "│  Tags:    {:width$} │",
                truncate(&tags.join(", "), w),
                width = w
            );
        }

        if let Some(n) = notes {
            println!("│  Notes:   {:width$} │", truncate(n, w), width = w);
        } else {
            println!("│  Notes:   {:width$} │", "-", width = w);
        }

        println!("└{:─<width$}┘", "", width = total_width);
        println!();
    }

    pub fn print_list_header(&self) {
        let tw = self.config.list_title_width;
        let tyw = self.config.list_type_width;
        let tgw = self.config.list_tags_width;
        println!(
            "{:<tw$} {:<tyw$} {:<tgw$}",
            "Title",
            "Type",
            "Tags",
            tw = tw,
            tyw = tyw,
            tgw = tgw
        );
        println!(
            "{:-<tw$} {:-^tyw$} {:-<tgw$}",
            "",
            "",
            "",
            tw = tw,
            tyw = tyw,
            tgw = tgw
        );
    }

    pub fn print_list_row(&self, title: &str, doc_type: &str, tags: &[String]) {
        let tw = self.config.list_title_width;
        let tyw = self.config.list_type_width;
        let tgw = self.config.list_tags_width;

        let tags_str = if tags.is_empty() {
            "-".to_string()
        } else {
            tags.join(", ")
        };

        println!(
            "{:<tw$} {:<tyw$} {:<tgw$}",
            truncate(title, tw),
            doc_type,
            truncate(&tags_str, tgw),
            tw = tw,
            tyw = tyw,
            tgw = tgw
        );
    }

    pub fn print_no_documents(&self) {
        println!("No documents found.");
    }

    pub fn print_search_header(&self) {
        println!(
            "{:<4} {:<tw$} {:<tyw$}",
            "ID",
            "Title",
            "Type",
            tw = self.config.search_title_width,
            tyw = self.config.search_type_width
        );
        println!(
            "{:-<4} {:-^tw$} {:-<tyw$}",
            "",
            "",
            "",
            tw = self.config.search_title_width,
            tyw = self.config.search_type_width
        );
    }

    pub fn print_search_row(&self, id: i64, title: &str, doc_type: &str) {
        println!(
            "{:<4} {:<tw$} {:<tyw$}",
            id,
            truncate(title, self.config.search_title_width),
            doc_type,
            tw = self.config.search_title_width,
            tyw = self.config.search_type_width
        );
    }

    pub fn print_no_matches(&self) {
        println!("No matching documents found.");
    }
}

fn truncate(s: &str, max_len: usize) -> String {
    if s.len() <= max_len {
        s.to_string()
    } else {
        format!("{}...", &s[..max_len.saturating_sub(3)])
    }
}
