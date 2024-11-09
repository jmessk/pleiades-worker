

pub struct Lambda {
    pub id: String,
    pub code: String,
}

pub struct Job {
    pub id: String,
    pub lambda: Lambda,
    pub input_id: String,
}
