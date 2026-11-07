use std::{
	fs::{self, File},
	io,
	path::{Path, PathBuf},
};

pub fn init() {
	let default_filter = if cfg!(debug_assertions) {
		"azusa=debug"
	} else {
		"azusa=info"
	};

	let mut builder =
		env_logger::Builder::from_env(env_logger::Env::default().default_filter_or(default_filter));

	if !cfg!(debug_assertions)
		&& let Some(dir) = log_dir()
		&& let Ok(file) = open_log_file(&dir)
	{
		builder.target(env_logger::Target::Pipe(Box::new(file)));
	}

	builder.init();

	let default_hook = std::panic::take_hook();
	std::panic::set_hook(Box::new(move |info| {
		log::error!("{info}\n{}", std::backtrace::Backtrace::force_capture());
		default_hook(info);
	}));
}

fn log_dir() -> Option<PathBuf> {
	let dirs = directories::ProjectDirs::from("", "", "Azusa")?;
	let dir = dirs.state_dir().unwrap_or(dirs.data_local_dir());
	Some(dir.to_path_buf())
}

fn open_log_file(dir: &Path) -> io::Result<File> {
	fs::create_dir_all(dir)?;

	let name = format!(
		"azusa_{}.log",
		jiff::Timestamp::now().strftime("%Y-%m-%dT%H-%M-%S")
	);

	File::create(dir.join(name))
}
