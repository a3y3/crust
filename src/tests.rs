mod tests {
    use dockertest::{Composition, Image};
    const IMAGE_NAME: &str = "crust";
    const NAME_ARG: &str = "--name";
    const NAME_VAL: &str = "node";
    const PORT_ARG: &str = "--port";
    const PORT_VAL: usize = 20000;
    #[test]
    fn test_immediate_successor() {
        let image = Image::with_repository(IMAGE_NAME);
        let num_containers: u8 = 3;
        let mut nodes_vec = Vec::new();
        for i in 0..num_containers {
            let mut name_val = NAME_VAL.to_string();
            name_val.push(i as char);
            let mut port_val = PORT_VAL.to_string();
            port_val.push(i as char);
            let node = Composition::with_image(image.clone()).with_cmd(vec![
                NAME_ARG.to_string(),
                name_val,
                PORT_ARG.to_string(),
                port_val,
            ]);
            nodes_vec.push(node);
        }
    }
}