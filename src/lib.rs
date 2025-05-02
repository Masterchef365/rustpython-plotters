use plotters::{
    chart::ChartBuilder, coord::Shift, prelude::{DrawingArea, DrawingBackend, IntoDrawingArea, PathElement}, series::LineSeries, style::{Color, IntoFont, BLACK, RED, WHITE}
};
use rustpython_vm::{builtins::PyModule, PyRef, VirtualMachine};


/// Python library export
pub fn make_module(vm: &VirtualMachine) -> PyRef<PyModule> {
    let module = pyplotter::make_module(vm);
    module
}

/// Dump the plotting commands plot since the last call
pub fn dump_commands() -> Vec<PlotCommand> {
    pyplotter::dump_commands()
}

#[rustpython_vm::pymodule]
pub mod pyplotter {
    use super::*;
    use std::borrow::BorrowMut;
    use std::cell::{LazyCell, RefCell};

    use rustpython_vm::builtins::{PyFloat, PyMappingProxy, PySequenceIterator, PyStr};
    use rustpython_vm::function::KwArgs;
    use rustpython_vm::object::MaybeTraverse;
    use rustpython_vm::protocol::{PyMapping, PySequence};
    use rustpython_vm::{PyObjectRef, PyResult, VirtualMachine};

    thread_local! {
        static COMMANDS: LazyCell<RefCell<Vec<PlotCommand>>> = LazyCell::new(RefCell::default);
    }

    #[pyfunction]
    fn plot(
        x: PyObjectRef,
        y: PyObjectRef,
        mut kwargs: KwArgs,
        vm: &VirtualMachine,
    ) -> PyResult<()> {
        let label: String = kwargs
            .pop_kwarg("label")
            .and_then(|label| label.downcast::<PyStr>().ok())
            .map(|py| py.to_string())
            .unwrap_or_default();

        let x = PySequence::try_protocol(&x, vm)?;
        let x: Vec<f32> = x.extract(|f| f.try_float(vm).map(|x| x.to_f64() as f32), vm)?;

        let y = PySequence::try_protocol(&y, vm)?;
        let y: Vec<f32> = y.extract(|f| f.try_float(vm).map(|y| y.to_f64() as f32), vm)?;

        COMMANDS.with(|reader| {
            (**reader).borrow_mut().push(PlotCommand::PlotXY {
                x,
                y,
                label,
            })
        });
        Ok(())
    }

    #[pyfunction]
    fn title(title: String, vm: &VirtualMachine) -> PyResult<()> {
        COMMANDS.with(|reader| {
            (**reader)
                .borrow_mut()
                .push(PlotCommand::Title(title))
        });

        Ok(())
    }


    #[pyfunction]
    fn xlim(left: f32, right: f32, vm: &VirtualMachine) -> PyResult<()> {
        COMMANDS.with(|reader| {
            (**reader)
                .borrow_mut()
                .push(PlotCommand::Xlim { left, right })
        });

        Ok(())
    }

    #[pyfunction]
    fn ylim(bottom: f32, top: f32, vm: &VirtualMachine) -> PyResult<()> {
        COMMANDS.with(|reader| {
            (**reader)
                .borrow_mut()
                .push(PlotCommand::Ylim { bottom, top })
        });

        Ok(())
    }

    pub(crate) fn dump_commands() -> Vec<PlotCommand> {
        COMMANDS.with(|r| std::mem::take(&mut *(**r).borrow_mut()))
    }
}

pub enum PlotCommand {
    Title(String),
    PlotXY {
        x: Vec<f32>,
        y: Vec<f32>,
        label: String,
    },
    Xlim {
        left: f32,
        right: f32,
    },
    Ylim {
        bottom: f32,
        top: f32,
    },
}

pub fn draw_plots<Db: DrawingBackend>(root: &DrawingArea<Db, Shift>, commands: &[PlotCommand]) -> Result<(), String> {
    root.fill(&WHITE).unwrap();

    let mut plot_left: f32 = -1.0;
    let mut plot_right: f32 = 1.0;
    let mut plot_top: f32 = 1.0;
    let mut plot_bottom: f32 = -1.0;
    let mut plot_title = String::new();

    for command in commands {
        match &command {
            PlotCommand::Title(title) => plot_title = title.clone(),
            PlotCommand::Ylim { bottom, top } => {
                plot_bottom = *bottom;
                plot_top = *top;
            }
            PlotCommand::Xlim { left, right } => {
                plot_left = *left;
                plot_right = *right;
            }
            PlotCommand::PlotXY { x, y, label } => {
                let mut chart = ChartBuilder::on(&root)
                    .caption(&plot_title, ("sans-serif", 25).into_font())
                    .margin(5)
                    .x_label_area_size(30)
                    .y_label_area_size(30)
                    .build_cartesian_2d(plot_left..plot_right, plot_bottom..plot_top)
                    .unwrap();

                chart.configure_mesh().draw().unwrap();

                let coords = 
                        x.iter()
                            .copied()
                            .zip(y.iter().copied())
                            .collect::<Vec<(f32, f32)>>();
                chart
                    .draw_series(LineSeries::new(coords, &RED))
                    .unwrap()
                    .label(label);
                    //.legend(|(x, y)| PathElement::new(vec![(x, y), (x + 20, y)], RED));
                //legend = false;

                chart
                    .configure_series_labels()
                    .background_style(&WHITE.mix(0.8))
                    .border_style(&BLACK)
                    .draw()
                    .unwrap();
            }
        }
    }

    root.present().unwrap();
    Ok(())
}
