use dockertest::{Composition, DockerTest, Image};
use gotham::helpers::http;
use gotham::test::TestServer;
use std::collections::HashMap;

const IMAGE_NAME: &str = "crust";
const NAME_ARG: &str = "--name";
const NAME_VAL: &str = "node";
const PORT_ARG: &str = "--port";
const PORT_VAL: usize = 20000;
use std::sync::{Arc, Mutex};
use std::{thread, time};
#[test]
fn hello_world_test() {
    // Define our test instance
    let mut test = DockerTest::new();

    // Construct the Composition to be added to the test.
    // A Composition is an Image configured with environment, arguments, StartPolicy, etc.,
    // seen as an instance of the Image prior to constructing the Container.
    let hello = Composition::with_repository("hello-world");

    // Populate the test instance.
    // The order of compositions added reflect the execution order (depending on StartPolicy).
    test.add_composition(hello);

    let has_ran: Arc<Mutex<bool>> = Arc::new(Mutex::new(false));
    let has_ran_test = has_ran.clone();
    test.run(|ops| async move {
        // A handle to operate on the Container.
        let _container = ops.handle("hello-world");

        // The container is in a running state at this point.
        // Depending on the Image, it may exit on its own (like this hello-world image)
        let mut ran = has_ran_test.lock().unwrap();
        *ran = true;
    });

    let ran = has_ran.lock().unwrap();
    assert!(*ran);
}

#[test]
fn test_immediate_successor() {
    let mut test = DockerTest::new();
    let image = Image::with_repository(IMAGE_NAME);
    let num_containers: u8 = 1;
    for i in 0..num_containers {
        // let mut name_val = NAME_VAL.to_string();
        // name_val.push(i as char);
        // let mut port_val = PORT_VAL.to_string();
        // port_val.push(i as char);
        let mut node = Composition::with_repository(IMAGE_NAME)
            .with_cmd(vec!["--init".to_string(), "-p".to_string(), "8000:8000".to_string()]);
        test.add_composition(node);
    }
    test.run(|ops| async {
        let resp = reqwest::get("http://localhost:8000/successor")
            .await
            .unwrap()
            .json::<HashMap<String, String>>().await.unwrap();
        println!("{:#?}", resp);
        assert_eq!(2, 3);
    });
}
