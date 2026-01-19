use mazer_types::implfuncs::ShowFunc;
use strum::IntoEnumIterator;

struct MazerLSP;

impl MazerLSP {
    fn get_all_nodes() -> Vec<String> {
        for f in ShowFunc::iter() {
            println!("{:?}", f);
        }

        vec![]
    }
}
