use rand::Rng;
use std::fs;

const V_REGISTERS_NUMBER: usize = 16;
const STACK_SIZE: usize = 16;
const RAM_SIZE: usize = 4096;
const INITIAL_ADDRESS: u16 = 0x200;

#[derive(Debug)]
struct Emulator {
    v_registers: [u8; V_REGISTERS_NUMBER],
    v_f_register: u8, // Not meant to be used by programs.
    i_register: u16,
    program_counter: u16,
    stack_pointer: u8,
    stack: [u16; STACK_SIZE],
    delay_timer_registry: usize,
    sound_timer_registry: usize,
    ram: [u8; RAM_SIZE],
}

fn pop_from_stack(emulator: &mut Emulator) -> u16 {
    emulator.stack_pointer -= 1;
    let element = emulator.stack[emulator.stack_pointer as usize];
    emulator.stack[emulator.stack_pointer as usize] = 0;
    return element;
}

fn push_to_stack(emulator: &mut Emulator, element: u16) {
    emulator.stack[emulator.stack_pointer as usize] = element;
    emulator.stack_pointer += 1;
}

fn read_rom(rom_name: &str) -> Vec<u8> {
    let path = format!("./roms/{}.ch8", rom_name);
    match fs::read(path) {
        Ok(content) => content,
        Err(error) => panic!("Error loading the ROM {:?}", error),
    }
}

fn load_rom_to_memory(emulator: &mut Emulator, data: &[u8]) {
    let start = INITIAL_ADDRESS as usize;
    let end = (INITIAL_ADDRESS as usize) + data.len();
    emulator.ram[start..end].copy_from_slice(data);
}

fn get_op_code(emulator: &Emulator) -> u16 {
    let higher_byte = emulator.ram[emulator.program_counter as usize] as u16;
    let lowe_byte = emulator.ram[(emulator.program_counter + 1) as usize] as u16;
    let op_code = (higher_byte << 8) | lowe_byte;
    op_code
}

fn parse_op_code(op_code: u16) -> (u16, u16, u16, u16) {
    let first = (op_code & 0xF000) >> 12;
    let second = (op_code & 0x0F00) >> 8;
    let third = (op_code & 0x00F0) >> 4;
    let fourth = op_code & 0x000F;
    return (first, second, third, fourth);
}

fn get_nnn(op_code: (u16, u16, u16, u16)) -> u16 {
    let (_, first, second, third) = op_code;
    let nnn = (first << 8) | (second << 4) | third;
    nnn
}

fn get_x(op_code: (u16, u16, u16, u16)) -> u16 {
    let (_, x, _, _) = op_code;
    x
}

fn get_kk(op_code: (u16, u16, u16, u16)) -> u16 {
    let (_, _, first, second) = op_code;
    let kk = (first << 4) | second;
    kk
}

fn get_y(op_code: (u16, u16, u16, u16)) -> u16 {
    let (_, _, y, _) = op_code;
    y
}

fn execute_op_code(emulator: &mut Emulator, op_code: (u16, u16, u16, u16)) {
    match op_code {
        // NOP
        (0, 0, 0, 0) => return,
        // CLS
        (0, 0, 0xE, 0) => unimplemented!("Clear screen"),
        // RET
        (0, 0, 0xE, 0xE) => {
            let address = pop_from_stack(emulator);
            emulator.program_counter = address;
        }
        // JP
        (1, _, _, _) => {
            let nnn = get_nnn(op_code);
            emulator.program_counter = nnn;
        }
        // CALL
        (2, _, _, _) => {
            let nnn = get_nnn(op_code);
            push_to_stack(emulator, emulator.program_counter);
            emulator.program_counter = nnn;
        }
        // SKIP if v[x] == kk
        (3, _, _, _) => {
            let x = get_x(op_code) as usize;
            let kk = get_kk(op_code) as u8;
            if emulator.v_registers[x] == kk {
                emulator.program_counter += 2;
            }
        }
        // SKIP if v[x] != kk
        (4, _, _, _) => {
            let x = get_x(op_code) as usize;
            let kk = get_kk(op_code) as u8;
            if emulator.v_registers[x] != kk {
                emulator.program_counter += 2;
            }
        }
        // SKIP if v[x] != kk
        (5, _, _, 0) => {
            let x = get_x(op_code) as usize;
            let y = get_y(op_code) as usize;
            if emulator.v_registers[x] == emulator.v_registers[y] {
                emulator.program_counter += 2;
            }
        }
        // PUT kk in v[x]
        (6, _, _, _) => {
            let x = get_x(op_code) as usize;
            let kk = get_kk(op_code) as u8;
            emulator.v_registers[x] = kk;
        }
        // ADD v[x] + kk
        (7, _, _, _) => {
            let x = get_x(op_code) as usize;
            let kk = get_kk(op_code) as u8;
            emulator.v_registers[x] += kk;
        }
        // SET v[x] = v[y]
        (8, _, _, 0) => {
            let x = get_x(op_code) as usize;
            let y = get_y(op_code) as usize;
            emulator.v_registers[x] = emulator.v_registers[y];
        }
        // OR v[x] | v[y]
        (8, _, _, 1) => {
            let x = get_x(op_code) as usize;
            let y = get_y(op_code) as usize;
            emulator.v_registers[x] |= emulator.v_registers[y];
        }
        // AND v[x] & v[y]
        (8, _, _, 2) => {
            let x = get_x(op_code) as usize;
            let y = get_y(op_code) as usize;
            emulator.v_registers[x] &= emulator.v_registers[y];
        }
        // XOR v[x] & v[y]
        (8, _, _, 3) => {
            let x = get_x(op_code) as usize;
            let y = get_y(op_code) as usize;
            emulator.v_registers[x] ^= emulator.v_registers[y];
        }
        // ADD v[x] + v[y]
        (8, _, _, 4) => {
            let x = get_x(op_code) as usize;
            let y = get_y(op_code) as usize;
            let vx = emulator.v_registers[x];
            let vy = emulator.v_registers[y];
            let (sum, overflow) = vx.overflowing_add(vy);
            emulator.v_registers[x] = sum;
            if overflow {
                emulator.v_registers[0xF] = 1
            }
        }
        // SUB v[x] - v[y]
        (8, _, _, 5) => {
            let x = get_x(op_code) as usize;
            let y = get_y(op_code) as usize;
            let vx = emulator.v_registers[x];
            let vy = emulator.v_registers[y];
            let (sub, borrow) = vx.overflowing_sub(vy);
            emulator.v_registers[x] = sub;
            if borrow {
                emulator.v_registers[0xF] = 1
            }
        }
        // SHR v[x] >> 1
        (8, _, _, 6) => {
            let x = get_x(op_code) as usize;
            let lsb = emulator.v_registers[x] & 1;
            emulator.v_registers[x] >>= 1;
            emulator.v_registers[0xF] = lsb;
        }
        // SUB v[x] = v[y] - v[x]
        (8, _, _, 7) => {
            let x = get_x(op_code) as usize;
            let y = get_y(op_code) as usize;
            let vx = emulator.v_registers[x];
            let vy = emulator.v_registers[y];
            let (sub, borrow) = vy.overflowing_sub(vx);
            emulator.v_registers[x] = sub;
            if borrow {
                emulator.v_registers[0xF] = 1
            }
        }
        // SHL v[x] << 1
        (8, _, _, 0xE) => {
            let x = get_x(op_code) as usize;
            let msb = (emulator.v_registers[x] >> 7) & 0xF0;
            emulator.v_registers[x] <<= 1;
            emulator.v_registers[0xF] = msb;
        }
        // SNE v[x] == v[y]
        (9, _, _, 0) => {
            let x = get_x(op_code) as usize;
            let y = get_y(op_code) as usize;
            if emulator.v_registers[x] != emulator.v_registers[y] {
                emulator.program_counter += 2;
            }
        }
        // SET vi = nnn
        (0xA, _, _, _) => {
            let nnn = get_nnn(op_code);
            emulator.i_register = nnn;
        }
        // JP V0 + nnn
        (0xB, _, _, _) => {
            let nnn = get_nnn(op_code);
            emulator.program_counter = emulator.v_registers[0] as u16 + nnn;
        }
        //RND Vx & kk
        (0xC, _, _, _) => {
            let x = get_x(op_code) as usize;
            let kk = get_kk(op_code) as u8;
            let rng: u8 = rand::thread_rng().gen();
            println!("{:?}", rng);
            emulator.v_registers[x] = rng & kk;
        }
        // Draw
        (0xD, _, _, _) => unimplemented!("Requires screen"),
        // SKIP if Vx is pressed
        (0xE, _, 9, 0xE) => unimplemented!("Requires keyboard"),
        // SKIP if Vx is not pressed
        (0xE, _, 0xA, 1) => unimplemented!("Requires keyboard"),
        // SET Vx to delay
        (0xF, _, 0, 7) => {
            let x = get_x(op_code) as usize;
            emulator.v_registers[x] = emulator.delay_timer_registry as u8
        }
        // Wait for key press
        (0xF, _, 0, 0xA) => unimplemented!("Requires keyboard"),
        // Set DT to Vx
        (0xF, _, 1, 5) => {
            let x = get_x(op_code) as usize;
            emulator.delay_timer_registry = emulator.v_registers[x] as usize;
        }
        // Set ST to Vx
        (0xF, _, 1, 8) => {
            let x = get_x(op_code) as usize;
            emulator.sound_timer_registry = emulator.v_registers[x] as usize;
        }
        // Set I = I + Vx.
        (0xF, _, 1, 0xE) => {
            let x = get_x(op_code) as usize;
            emulator.i_register = emulator
                .i_register
                .wrapping_add(emulator.v_registers[x] as u16);
        }
        // Set BCD = Vx in I address
        (0xF, _, 3, 3) => {
            let x = get_x(op_code) as usize;
            let vx = emulator.v_registers[x];
            let i = emulator.i_register as usize;
            let hundreds = vx / 100;
            let tens = (vx / 10) % 10;
            let ones = vx % 10;

            emulator.ram[i] = hundreds;
            emulator.ram[i + 1] = tens;
            emulator.ram[i + 2] = ones;
        }
        // Store registers in memory
        (0xF, _, 5, 5) => {
            let x = get_x(op_code) as usize;
            for i in 0..=x {
                emulator.ram[i + emulator.i_register as usize] = emulator.v_registers[i];
            }
        }
        // Loads memory into registers
        (0xF, _, 6, 5) => {
            let x = get_x(op_code) as usize;
            for i in 0..=x {
                emulator.v_registers[i] = emulator.ram[i + emulator.i_register as usize];
            }
        }

        (_, _, _, _) => unimplemented!("Unimplemented opcode: {:?}", op_code),
    }
}

fn main() {
    let mut emulator = Emulator {
        v_registers: [0; V_REGISTERS_NUMBER],
        v_f_register: 0,
        i_register: 0,
        program_counter: INITIAL_ADDRESS,
        stack_pointer: 0,
        stack: [0; STACK_SIZE],
        delay_timer_registry: 0,
        sound_timer_registry: 0,
        ram: [0; RAM_SIZE],
    };
    // TODO: Implement main loop
    emulator.v_registers[0] = 127;
    emulator.v_registers[1] = 10;
    emulator.ram[0] = 1;
    emulator.ram[1] = 2;
    let data = read_rom("pong");
    load_rom_to_memory(&mut emulator, &data);
    // let op_code = get_op_code(&emulator);
    let op_code = 0xF165;
    let parsed_code = parse_op_code(op_code);
    execute_op_code(&mut emulator, parsed_code);
    println!("{:x?}", emulator);
    // println!("{:x?}", op_code);
    // println!("{:x?}", parsed_code);
    // println!("{:x?}", data);
}
