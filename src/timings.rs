use std::time::Instant;

use comfy_table::Table;

pub struct Timings {
    points: Vec<(String, f64)>,
    last: Instant,
}

impl Timings {
    pub fn start() -> Self {
        Self {
            points: vec![],
            last: Instant::now(),
        }
    }

    pub fn lap(&mut self, label: &str) {
        let now = Instant::now();
        let taken = now.duration_since(self.last).as_secs_f64() * 1000.0;
        self.last = now;
        self.points.push((label.to_string(), taken));
    }

    pub fn append(&mut self, other: Self, prefix: &str) {
        if other.points.is_empty() {
            self.lap(prefix);
        }

        for (label, time) in other.points {
            self.points.push((format!("{prefix}/{label}"), time));
        }
    }

    pub fn report(&self) -> String {
        let total = self.points.iter().map(|x| x.1).sum::<f64>();
        let mut table = Table::new();
        table.set_header(vec!["Category", "Time (MS)", "%"]);
        for (label, time) in &self.points {
            table.add_row(vec![
                label.to_string(),
                format!("{time:.4}"),
                format!("{:.4}", time * 100.0 / total),
            ]);
        }

        table.add_row(vec![
            "Total".to_string(),
            format!("{total:.4}"),
            "100%".to_string(),
        ]);

        table.to_string()
    }
}
