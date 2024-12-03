use anyhow::anyhow;
use geo_json_multiblock_merger::run;
use rust_i18n::t;

rust_i18n::i18n!("locales");

fn main() {
    match choose_lang(){
        Ok(_) => {
            println!(
                "\n{}\n{}: {}\n",
                t!("program.desc"),
                t!("program.version.str"),
                env!("CARGO_PKG_VERSION")
            );

            let curr_dir = std::env::current_dir();
            match curr_dir {
                Ok(dir) => loop {
                    if let Err(e) = run(&dir) {
                        println!("{}", e);
                        break;
                    }
                },
                Err(e) => {
                    println!("{}: {}", t!("msg.get_run_dic"), e);
                }
            }
        }
        Err(e) => {
            eprintln!("{}", e);
        }
    }

    let _ = inquire::Text::new(t!("msg.press_any_key_exit").as_ref()).prompt();
}

fn choose_lang() -> anyhow::Result<()> {
    println!("choose lang:");
    let locales = rust_i18n::available_locales!();
    let select = inquire::Select::new("choose language: ", locales.clone()).prompt();
    match select {
        Ok(lang) => {
            rust_i18n::set_locale(lang);
            Ok(())
        }
        Err(e) => Err(anyhow!(e)),
    }
}
