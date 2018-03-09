use std::collections::{BTreeMap, HashMap};
use std::path::PathBuf;
use std::str::FromStr;

use minidom::{Error, ErrorKind, Element};
use slog::Logger;

use utils::parse::{attr_map, attr_parse, attr_parse_hex, FromElem};
use utils::ResultLogExt;

#[derive(Debug, Clone, Serialize, Deserialize)]
enum Core {
    CortexM0,
    CortexM0Plus,
    CortexM1,
    CortexM3,
    CortexM4,
    CortexM7,
    CortexM23,
    CortexM33,
    SC000,
    SC300,
    ARMV8MBL,
    ARMV8MML,
    CortexR4,
    CortexR5,
    CortexR7,
    CortexR8,
    CortexA5,
    CortexA7,
    CortexA8,
    CortexA9,
    CortexA15,
    CortexA17,
    CortexA32,
    CortexA35,
    CortexA53,
    CortexA57,
    CortexA72,
    CortexA73,
}

impl FromStr for Core {
    type Err = Error;
    fn from_str(from: &str) -> Result<Self, Error> {
        match from {
            "Cortex-M0" =>  Ok(Core::CortexM0),
            "Cortex-M0+" => Ok(Core::CortexM0Plus),
            "Cortex-M1" =>  Ok(Core::CortexM1),
            "Cortex-M3" =>  Ok(Core::CortexM3),
            "Cortex-M4" =>  Ok(Core::CortexM4),
            "Cortex-M7" =>  Ok(Core::CortexM7),
            "Cortex-M23" => Ok(Core::CortexM23),
            "Cortex-M33" => Ok(Core::CortexM33),
            "SC000" =>      Ok(Core::SC000),
            "SC300" =>      Ok(Core::SC300),
            "ARMV8MBL" =>   Ok(Core::ARMV8MBL),
            "ARMV8MML" =>   Ok(Core::ARMV8MML),
            "Cortex-R4" =>  Ok(Core::CortexR4),
            "Cortex-R5" =>  Ok(Core::CortexR5),
            "Cortex-R7" =>  Ok(Core::CortexR7),
            "Cortex-R8" =>  Ok(Core::CortexR8),
            "Cortex-A5" =>  Ok(Core::CortexA5),
            "Cortex-A7" =>  Ok(Core::CortexA7),
            "Cortex-A8" =>  Ok(Core::CortexA8),
            "Cortex-A9" =>  Ok(Core::CortexA9),
            "Cortex-A15" => Ok(Core::CortexA15),
            "Cortex-A17" => Ok(Core::CortexA17),
            "Cortex-A32" => Ok(Core::CortexA32),
            "Cortex-A35" => Ok(Core::CortexA35),
            "Cortex-A53" => Ok(Core::CortexA53),
            "Cortex-A57" => Ok(Core::CortexA57),
            "Cortex-A72" => Ok(Core::CortexA72),
            "Cortex-A73" => Ok(Core::CortexA73),
            unknown => Err(err_msg!("Unknown core {}", unknown)),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Processor {
    core: Core
}

impl Processor {
    fn merge(self, _parent: &Self) -> Self {
        self
    }
}

impl FromElem for Processor {
    fn from_elem(e: &Element, _: &Logger) -> Result<Self, Error> {
        Ok(Processor{
            core: attr_parse(e, "Dcore", "processor")?
        })
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum Processors {
    Symmetric(Processor),
    Asymmetric(BTreeMap<String, Processor>),
}

impl Processors{
    fn merge(self, parent: &Option<Self>) -> Result<Self, Error> {
        match self {
            Processors::Symmetric(me) =>
                match parent {
                    &Some(Processors::Symmetric(ref single_core)) =>
                        Ok(Processors::Symmetric(me.merge(single_core))),
                    &Some(Processors::Asymmetric(_)) =>
                        Err(err_msg!("Tried to merge symmetric and asymmetric processors")),
                    &None => Ok(Processors::Symmetric(me)),
                },
            Processors::Asymmetric(mut me) =>
                match parent {
                    &Some(Processors::Symmetric(_)) =>
                        Err(err_msg!("Tried to merge asymmetric and symmetric processors")),
                    &Some(Processors::Asymmetric(ref par_map)) => {
                        me.extend(par_map.iter().map(|(k, v)| (k.clone(), v.clone())));
                        Ok(Processors::Asymmetric(me))
                    },
                    &None => Ok(Processors::Asymmetric(me)),
                },
        }
    }

    fn merge_into(&mut self, other: Self) {
        match self {
            &mut Processors::Symmetric(_) => (),
            &mut Processors::Asymmetric(ref mut me) =>
                match other {
                    Processors::Symmetric(_) => (),
                    Processors::Asymmetric(more) => me.extend(more.into_iter()),
                }
        }
    }
}

impl FromElem for Processors {
    fn from_elem(e: &Element, l: &Logger) -> Result<Self, Error> {
        Ok(match e.attr("Pname") {
            Some(name) => Processors::Asymmetric(Some((name.to_string(), Processor::from_elem(e, l)?)).into_iter().collect()),
            None => Processors::Symmetric(Processor::from_elem(e, l)?)
        })
    }
}


#[derive(Debug, Clone, Serialize, Deserialize)]
struct MemoryPermissions {
    read: bool,
    write: bool,
    execute: bool,
}

impl MemoryPermissions {
    fn from_str(input: &str) -> Self {
        let mut ret = MemoryPermissions {
            read: false,
            write: false,
            execute: false,
        };
        for c in input.chars() {
            match c {
                'r' => ret.read = true,
                'w' => ret.write = true,
                'x' => ret.execute = true,
                _ => (),
            }
        }
        ret
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct Memory {
    access: MemoryPermissions,
    start: u64,
    size: u64,
    startup: bool,
}

struct MemElem(String, Memory);

impl FromElem for MemElem {
    fn from_elem(e: &Element, _l: &Logger) -> Result<Self, Error> {
        let access = e.attr("id")
            .map(|memtype| if memtype.contains("ROM") {
                "rx"
            } else if memtype.contains("RAM") {
                "rw"
            } else {
                ""
            })
            .or_else(|| e.attr("access"))
            .map(|memtype| MemoryPermissions::from_str(memtype))
            .unwrap();
        let name = e.attr("id")
            .or_else(|| e.attr("name"))
            .map(|s| s.to_string())
            .ok_or_else(|| err_msg!("No name found for memory"))?;
        let start = attr_parse_hex(e, "start", "memory")?;
        let size = attr_parse_hex(e, "size", "memory")?;
        let startup = attr_parse(e, "startup", "memory").unwrap_or_default();
        Ok(MemElem(
            name,
            Memory {
                access,
                start,
                size,
                startup,
            },
        ))
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Memories(HashMap<String, Memory>);

fn merge_memories(lhs: Memories, rhs: &Memories) -> Memories {
    let rhs: Vec<_> = rhs.0
        .iter()
        .filter_map(|(k, v)| if lhs.0.contains_key(k) {
            None
        } else {
            Some((k.clone(), v.clone()))
        })
        .collect();
    let mut lhs = lhs;
    lhs.0.extend(rhs);
    lhs
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Algorithm {
    file_name: PathBuf,
    start: u64,
    size: u64,
    default: bool,
}

impl FromElem for Algorithm {
    fn from_elem(e: &Element, _l: &Logger) -> Result<Self, Error> {
        Ok(Self {
            file_name: attr_map(e, "name", "algorithm")?,
            start: attr_parse_hex(e, "start", "algorithm")?,
            size: attr_parse_hex(e, "size", "algorithm")?,
            default: attr_parse(e, "default", "algorithm").unwrap_or_default(),
        })
    }
}

#[derive(Debug)]
struct DeviceBuilder<'dom> {
    name: Option<&'dom str>,
    algorithms: Vec<Algorithm>,
    memories: Memories,
    processor: Option<Processors>,
}

#[derive(Debug, Serialize)]
pub struct Device {
    pub name: String,
    pub memories: Memories,
    pub algorithms: Vec<Algorithm>,
    pub processor: Processors,
}

impl<'dom> DeviceBuilder<'dom> {
    fn from_elem(e: &'dom Element) -> Self {
        let memories = Memories(HashMap::new());
        DeviceBuilder {
            name: e.attr("Dname").or_else(|| e.attr("Dvariant")),
            memories,
            algorithms: Vec::new(),
            processor: None,
        }
    }

    fn build(self) -> Result<Device, Error> {
        let name = self.name.map(|s| s.into()).ok_or_else(|| {
            err_msg!("Device found without a name")
        })?;
        Ok(Device {
            processor: self.processor.ok_or_else(||{
                err_msg!("Device {} found without a processor", name)
            })?,
            name,
            memories: self.memories,
            algorithms: self.algorithms,
        })
    }

    fn add_parent(mut self, parent: &Self) -> Result<Self, Error> {
        self.algorithms.extend_from_slice(&parent.algorithms);
        Ok(Self {
            name: self.name.or(parent.name),
            algorithms: self.algorithms,
            memories: merge_memories(self.memories, &parent.memories),
            processor: match self.processor {
                Some(old_proc) => Some(old_proc.merge(&parent.processor)?),
                None => parent.processor.clone(),
            }
        })
    }

    fn add_processor(&mut self, processor: Processors) -> &mut Self {
        match self.processor {
            None => self.processor = Some(processor),
            Some(ref mut origin) => origin.merge_into(processor),
        };
        self
    }

    fn add_memory(&mut self, MemElem(name, mem): MemElem) -> &mut Self {
        self.memories.0.insert(name, mem);
        self
    }

    fn add_algorithm(&mut self, alg: Algorithm) -> &mut Self {
        self.algorithms.push(alg);
        self
    }
}

fn parse_device<'dom>(e: &'dom Element, l: &Logger) -> Vec<DeviceBuilder<'dom>> {
    let mut device = DeviceBuilder::from_elem(e);
    let variants = e.children()
        .filter_map(|child| match child.name() {
            "variant" => Some(DeviceBuilder::from_elem(child)),
            "memory" => {
                FromElem::from_elem(child, l)
                    .ok_warn(l)
                    .map(|mem| device.add_memory(mem));
                None
            }
            "algorithm" => {
                FromElem::from_elem(child, l)
                    .ok_warn(l)
                    .map(|alg| device.add_algorithm(alg));
                None
            }
            "processor" => {
                FromElem::from_elem(child, l)
                    .ok_warn(l)
                    .map(|prc| device.add_processor(prc));
                None
            }
            _ => None,
        })
        .collect::<Vec<_>>();
    if variants.is_empty() {
        vec![device]
    } else {
        variants
            .into_iter()
            .flat_map(|bld| bld.add_parent(&device).ok_warn(l))
            .collect()
    }
}

fn parse_sub_family<'dom>(e: &'dom Element, l: &Logger) -> Vec<DeviceBuilder<'dom>> {
    let mut sub_family_device = DeviceBuilder::from_elem(e);
    let devices = e.children()
        .flat_map(|child| match child.name() {
            "device" => parse_device(child, l),
            "memory" => {
                FromElem::from_elem(child, l)
                    .ok_warn(l)
                    .map(|mem| sub_family_device.add_memory(mem));
                Vec::new()
            }
            "algorithm" => {
                FromElem::from_elem(child, l)
                    .ok_warn(l)
                    .map(|alg| sub_family_device.add_algorithm(alg));
                Vec::new()
            }
            "processor" => {
                FromElem::from_elem(child, l)
                    .ok_warn(l)
                    .map(|prc| sub_family_device.add_processor(prc));
                Vec::new()
            }
            _ => Vec::new(),
        })
        .collect::<Vec<_>>();
    devices
        .into_iter()
        .flat_map(|bldr| bldr.add_parent(&sub_family_device).ok_warn(l))
        .collect()
}

fn parse_family<'dom>(e: &Element, l: &Logger) -> Result<Vec<Device>, Error> {
    let mut family_device = DeviceBuilder::from_elem(e);
    let all_devices = e.children()
        .flat_map(|child| match child.name() {
            "subFamily" => parse_sub_family(child, &l),
            "device" => parse_device(child, &l),
            "memory" => {
                FromElem::from_elem(child, l)
                    .ok_warn(l)
                    .map(|mem| family_device.add_memory(mem));
                Vec::new()
            }
            "algorithm" => {
                FromElem::from_elem(child, l)
                    .ok_warn(l)
                    .map(|alg| family_device.add_algorithm(alg));
                Vec::new()
            }
            "processor" => {
                FromElem::from_elem(child, l)
                    .ok_warn(l)
                    .map(|prc| family_device.add_processor(prc));
                Vec::new()
            }
            _ => Vec::new(),
        })
        .collect::<Vec<_>>();
    all_devices
        .into_iter()
        .map(|bldr| bldr.add_parent(&family_device).and_then(|dev| dev.build()))
        .collect()
}

#[derive(Default, Serialize)]
pub struct Devices(pub(crate) HashMap<String, Device>);

impl FromElem for Devices {
    fn from_elem(e: &Element, l: &Logger) -> Result<Self, Error> {
        e.children()
            .fold(
                Ok(HashMap::new()),
                |res, c| match (res, parse_family(c, l)) {
                    (Ok(mut devs), Ok(add_this)) => {
                        devs.extend(add_this.into_iter().map(|dev| (dev.name.clone(), dev)));
                        Ok(devs)
                    }
                    (Ok(_), Err(e)) => Err(e),
                    (Err(e), Ok(_)) => Err(e),
                    (Err(e), Err(_)) => Err(e),
                },
            )
            .map(Devices)
    }
}
