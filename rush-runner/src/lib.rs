mod result;

use std::collections::HashMap;
use std::ffi::CString;
use std::os::fd::FromRawFd;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};

use nix::fcntl::{FcntlArg, FdFlag};
use nix::sys::signal::{self, SigHandler, Signal};
use nix::sys::wait::{WaitPidFlag, WaitStatus, waitpid};
use nix::unistd::{ForkResult, Pid, tcgetpgrp, tcsetpgrp};
use rush_parser::{Ast, DisplayAst, SimpleCommand};

pub use crate::result::Error;
use crate::result::Result;

static JOBS_UPDATED: AtomicBool = AtomicBool::new(false);

extern "C" fn sigchld_handler(_: i32) {
    JOBS_UPDATED.store(true, Ordering::Relaxed);
}

#[derive(Debug, Clone)]
pub enum JobStatus {
    Running,
    Stopped,
    Done(i32), // exit code
}

#[derive(Debug, Clone)]
pub struct Job {
    pub id: u32,
    pub process_group_id: Pid,
    pub command: String,
    pub status: JobStatus,
    pub is_foreground: bool,
}

impl Job {
    pub fn new(job_id: u32, process_group_id: Pid, command: String, is_foreground: bool) -> Self {
        Self {
            id: job_id,
            process_group_id,
            command,
            status: JobStatus::Running,
            is_foreground,
        }
    }
}

pub struct ExecCtx<'ctx> {
    pub source: &'ctx str,
    pub jobs: Arc<Mutex<HashMap<u32, Job>>>,
    pub next_job_id: Arc<Mutex<u32>>,
    pub shell_pgid: Pid,
    pub shell_terminal: i32,
}

pub fn init_shell() -> Result<(Pid, i32), Box<dyn std::error::Error>> {
    let shell_terminal = nix::libc::STDIN_FILENO;
    let stdin_fd = unsafe { std::os::fd::BorrowedFd::borrow_raw(shell_terminal) };
    let shell_is_interactive = nix::unistd::isatty(stdin_fd)?;

    if !shell_is_interactive {
        return Ok((nix::unistd::getpid(), shell_terminal));
    }

    // ignore interactive and job-control signals
    unsafe {
        signal::signal(Signal::SIGINT, SigHandler::SigIgn)?;
        signal::signal(Signal::SIGQUIT, SigHandler::SigIgn)?;
        signal::signal(Signal::SIGTSTP, SigHandler::SigIgn)?;
        signal::signal(Signal::SIGTTIN, SigHandler::SigIgn)?;
        signal::signal(Signal::SIGTTOU, SigHandler::SigIgn)?;
        signal::signal(Signal::SIGCHLD, SigHandler::Handler(sigchld_handler))?;
    }

    let shell_pgid = nix::unistd::getpid();
    if nix::unistd::setpgid(shell_pgid, shell_pgid).is_err() {
        eprintln!("Couldn't put the shell in its own process group");
    }

    let stdin_fd = unsafe { std::os::fd::BorrowedFd::borrow_raw(shell_terminal) };
    loop {
        let current_pgrp = tcgetpgrp(stdin_fd)?;
        if current_pgrp == shell_pgid {
            break;
        }
        nix::sys::signal::kill(nix::unistd::Pid::from_raw(-shell_pgid.as_raw()), Signal::SIGTTIN)?;
    }

    tcsetpgrp(stdin_fd, shell_pgid)?; // set shell as the foreground process group
    Ok((shell_pgid, shell_terminal))
}

pub fn update_job_statuses(jobs: Arc<Mutex<HashMap<u32, Job>>>) {
    if !JOBS_UPDATED.swap(false, Ordering::Relaxed) {
        return;
    }

    let mut jobs_lock = jobs.lock().unwrap();
    let mut completed_jobs = Vec::new();

    for (job_id, job) in jobs_lock.iter_mut() {
        if matches!(job.status, JobStatus::Done(_)) {
            continue;
        }

        match waitpid(
            Some(job.process_group_id),
            Some(WaitPidFlag::WNOHANG | WaitPidFlag::WUNTRACED),
        ) {
            Ok(WaitStatus::Exited(_, exit_code)) => {
                job.status = JobStatus::Done(exit_code);
                completed_jobs.push(*job_id);
            }
            Ok(WaitStatus::Signaled(_, signal, _)) => {
                job.status = JobStatus::Done(128 + signal as i32);
                completed_jobs.push(*job_id);
            }
            Ok(WaitStatus::Stopped(_, _)) => {
                job.status = JobStatus::Stopped;
            }
            Ok(WaitStatus::Continued(_)) => {
                job.status = JobStatus::Running;
            }
            _ => {} // still running or error
        }
    }

    for job_id in completed_jobs {
        if let Some(job) = jobs_lock.get(&job_id) {
            match &job.status {
                JobStatus::Done(0) => println!("[{}] Done                    {}", job_id, job.command),
                JobStatus::Done(code) => println!("[{}] Exit {}                {}", job_id, code, job.command),
                _ => {}
            }
        }
    }
}

pub fn execute(ctx: &mut ExecCtx<'_>, commands: Ast) -> Result<()> {
    match commands {
        Ast::Command(cmd) => execute_command(ctx, cmd)?,
        Ast::Pipeline(cmds) => execute_pipeline(ctx, cmds)?,
        Ast::BackgroundJob(ast) => execute_background_job(ctx, *ast)?,
        Ast::Sequence(seq) => {
            for cmd in seq {
                execute(ctx, cmd)?
            }
        }
    }

    Ok(())
}

fn execute_background_job(ctx: &mut ExecCtx<'_>, ast: Ast) -> Result<()> {
    let job_id = {
        let mut next_id = ctx.next_job_id.lock().unwrap();
        let id = *next_id;
        *next_id += 1;
        id
    };

    match unsafe { nix::unistd::fork() }? {
        ForkResult::Parent { child, .. } => {
            let job = Job::new(job_id, child, ast.to_string(ctx.source), false);
            ctx.jobs.lock().unwrap().insert(job_id, job);
            println!("[{job_id}] {child}");
        }
        ForkResult::Child => {
            let child_pid = nix::unistd::getpid();
            let _ = nix::unistd::setpgid(child_pid, child_pid);
            _ = execute(ctx, ast); // NOTE: maybe if execute fails we need to do something... not sure
            std::process::exit(0);
        }
    }

    Ok(())
}

fn execute_command(ctx: &mut ExecCtx<'_>, cmd: SimpleCommand) -> Result<()> {
    let program_name = cmd.program.slice(ctx.source);
    let program_name_cstr = CString::new(program_name).unwrap();
    let program_args_cstr = std::iter::once(program_name_cstr.clone())
        .chain(cmd.args.iter().map(|s| CString::new(s.slice(ctx.source)).unwrap()))
        .collect::<Vec<_>>();

    match unsafe { nix::unistd::fork() }? {
        ForkResult::Parent { child } => _ = nix::sys::wait::waitpid(child, None),
        ForkResult::Child => {
            let _ = nix::unistd::execvp(&program_name_cstr, &program_args_cstr);
            eprintln!("rush: command not found: {program_name}");
            std::process::exit(127);
        }
    }

    Ok(())
}

fn execute_pipeline(ctx: &mut ExecCtx<'_>, commands: Vec<SimpleCommand>) -> Result<()> {
    if commands.is_empty() {
        return Ok(());
    }

    let mut programs = vec![];
    for command in commands.iter() {
        let program_name = command.program.slice(ctx.source);
        let program_args = command.args.iter().map(|arg| arg.slice(ctx.source)).collect::<Vec<_>>();

        let mut program_command = vec![];

        program_command.push(CString::new(program_name).unwrap());
        program_args
            .into_iter()
            .for_each(|arg| program_command.push(CString::new(arg).unwrap()));

        programs.push(program_command);
    }

    let mut pipes = vec![];
    for _ in 0..(commands.len() - 1) {
        let (read, write) = nix::unistd::pipe().unwrap();
        nix::fcntl::fcntl(&read, FcntlArg::F_SETFD(FdFlag::FD_CLOEXEC)).ok();
        nix::fcntl::fcntl(&write, FcntlArg::F_SETFD(FdFlag::FD_CLOEXEC)).ok();
        pipes.push((read, write));
    }

    let mut child_pids = vec![];
    let mut process_group_id: Option<nix::unistd::Pid> = None;

    for idx in 0..commands.len() {
        match unsafe { nix::unistd::fork() }? {
            ForkResult::Child => {
                let child_pid = nix::unistd::getpid();
                let target_process_group_id = process_group_id.unwrap_or(child_pid);
                let mut stdin_fd = unsafe { std::os::fd::OwnedFd::from_raw_fd(nix::libc::STDIN_FILENO) };
                let mut stdout_fd = unsafe { std::os::fd::OwnedFd::from_raw_fd(nix::libc::STDOUT_FILENO) };

                let _ = nix::unistd::setpgid(nix::unistd::Pid::from_raw(0), target_process_group_id);

                // if this is not the first process then we need to wire its stdin
                if idx > 0 {
                    let (read, _) = &pipes[idx - 1];
                    _ = nix::unistd::dup2(read, &mut stdin_fd);
                }

                // if this is not the last process, we need to wire its stdout
                if idx + 1 < commands.len() {
                    let (_, write) = &pipes[idx];
                    _ = nix::unistd::dup2(write, &mut stdout_fd);
                }

                // close all pipe fds in child (they were dup'd)
                for (rfd, wfd) in &pipes {
                    let _ = nix::unistd::close(rfd.try_clone().unwrap());
                    let _ = nix::unistd::close(wfd.try_clone().unwrap());
                }

                let program_name = &programs[idx][0];
                let program_args = &programs[idx];

                let _ = nix::unistd::execvp(program_name, program_args);
                std::process::exit(127);
            }
            ForkResult::Parent { child } => {
                if process_group_id.is_none() {
                    process_group_id = Some(child);
                }

                let target_process_group_id = process_group_id.unwrap();
                let _ = nix::unistd::setpgid(child, target_process_group_id);
                child_pids.push(child);
            }
        }
    }

    // close all remaining pipe file descriptors in parent
    for (rfd, wfd) in pipes {
        let _ = nix::unistd::close(rfd);
        let _ = nix::unistd::close(wfd);
    }

    for pid in &child_pids {
        loop {
            match nix::sys::wait::waitpid(Some(*pid), None) {
                Ok(WaitStatus::Exited(_, _)) => break,
                Ok(WaitStatus::Signaled(_, _, _)) => break,
                Ok(WaitStatus::Stopped(_, _)) => continue,
                Ok(WaitStatus::Continued(_)) => continue,
                Ok(WaitStatus::StillAlive) => continue,
                Err(_) => break,
            }
        }
    }

    Ok(())
}
